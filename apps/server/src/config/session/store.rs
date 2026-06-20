use std::{collections::HashMap, str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, Utc};
use models::entity::session;
use sea_orm::{prelude::*, sea_query::Expr, DatabaseConnection, Set};
use stump_core::config::StumpConfig;
use time::OffsetDateTime;
use tokio::time::MissedTickBehavior;
use tower_sessions::{
	session::{Id, Record},
	session_store::{self, ExpiredDeletion},
	SessionStore,
};

use super::SESSION_USER_KEY;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
	#[error("{0}")]
	DbError(#[from] sea_orm::error::DbErr),
	#[error("Session not found")]
	NotFound,
	#[error("An error occurred while serializing or deserializing session data: {0}")]
	SerdeError(#[from] serde_json::Error),
	#[error("Failed to decode session data")]
	DecodeFailed,
}

impl From<SessionError> for session_store::Error {
	fn from(error: SessionError) -> Self {
		match error {
			SessionError::NotFound => {
				session_store::Error::Backend("Session not found".to_string())
			},
			SessionError::DbError(e) => session_store::Error::Backend(e.to_string()),
			SessionError::SerdeError(e) => session_store::Error::Decode(e.to_string()),
			SessionError::DecodeFailed => {
				session_store::Error::Decode("Failed to decode session data".to_string())
			},
		}
	}
}

#[derive(Clone, Debug)]
pub struct StumpSessionStore {
	conn: Arc<DatabaseConnection>,
	config: Arc<StumpConfig>,
}

impl StumpSessionStore {
	pub fn new(conn: Arc<DatabaseConnection>, config: Arc<StumpConfig>) -> Self {
		Self { conn, config }
	}

	pub async fn continuously_delete_expired(self, period: tokio::time::Duration) {
		let mut interval = tokio::time::interval(period);
		interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
		loop {
			interval.tick().await;
			if let Err(error) = self.delete_expired().await {
				tracing::error!(?error, "Failed to delete expired sessions");
			} else {
				tracing::trace!("Deleted expired sessions");
			}
			tracing::trace!("Waiting for next session cleanup interval...");
		}
	}

	/// Slide a live session's server-side expiry forward to `now + session_ttl`.
	///
	/// The auth middleware only reads sessions, so tower-sessions never marks them
	/// dirty and `SessionStore::save` (INSERT-only here) is never called after login
	/// — which froze `expiry_time` at `login + ttl` and logged active users out on a
	/// fixed schedule. This bumps the row directly. The update is throttled in SQL so
	/// a given session is touched at most once per `SLIDE_THROTTLE_SECS` (the filter
	/// matches only rows whose expiry is more than that far from a fresh full window),
	/// and it never resurrects an already-expired row (`expiry_time > now`).
	pub async fn touch_expiry(&self, session_id: &str) -> Result<(), SessionError> {
		// Touch a session at most once per hour, not on every request.
		const SLIDE_THROTTLE_SECS: i64 = 3600;

		let now = Utc::now();
		let ttl = self.config.session_ttl;
		let new_expiry: DateTime<FixedOffset> = (now + Duration::seconds(ttl)).into();
		let throttle_before: DateTime<FixedOffset> =
			(now + Duration::seconds(ttl - SLIDE_THROTTLE_SECS)).into();
		let now_offset: DateTime<FixedOffset> = now.into();

		session::Entity::update_many()
			.col_expr(session::Column::ExpiryTime, Expr::value(new_expiry))
			.filter(
				session::Column::SessionId
					.eq(session_id)
					.and(session::Column::ExpiryTime.lt(throttle_before))
					.and(session::Column::ExpiryTime.gt(now_offset)),
			)
			.exec(self.conn.as_ref())
			.await
			.map_err(SessionError::DbError)?;

		Ok(())
	}
}

#[async_trait]
impl ExpiredDeletion for StumpSessionStore {
	#[tracing::instrument(skip(self))]
	async fn delete_expired(&self) -> session_store::Result<()> {
		let affected_rows = session::Entity::delete_many()
			.filter(
				session::Column::ExpiryTime.lt::<DateTimeWithTimeZone>(Utc::now().into()),
			)
			.exec(self.conn.as_ref())
			.await
			.map_err(SessionError::DbError)?
			.rows_affected;

		tracing::trace!(?affected_rows, "Deleted expired sessions");

		Ok(())
	}
}

#[async_trait]
impl SessionStore for StumpSessionStore {
	#[tracing::instrument(skip(self))]
	async fn save(&self, record: &Record) -> session_store::Result<()> {
		let expiry_time: DateTime<FixedOffset> =
			(Utc::now() + Duration::seconds(self.config.session_ttl)).into();

		let user_id = record
			.data
			.get(SESSION_USER_KEY)
			.and_then(|v| v.as_str())
			.ok_or(SessionError::NotFound)?;
		let session_id = record.id.to_string();
		tracing::trace!(session_id, ?user_id, "Saving session");

		let active_model = session::ActiveModel {
			session_id: Set(session_id.clone()),
			user_id: Set(user_id.to_string()),
			expiry_time: Set(expiry_time),
			created_at: Set(DateTimeWithTimeZone::from(Utc::now())),
			..Default::default()
		};
		let _session = active_model
			.insert(self.conn.as_ref())
			.await
			.map_err(SessionError::DbError)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
		tracing::trace!(?session_id, "Loading session");

		let record = session::Entity::find()
			.filter(session::Column::SessionId.eq(session_id.to_string()).and(
				session::Column::ExpiryTime.gt::<DateTimeWithTimeZone>(Utc::now().into()),
			))
			.one(self.conn.as_ref())
			.await
			.map_err(SessionError::DbError)?;

		if let Some(result) = record {
			tracing::trace!("Found session record");
			Ok(Some(Record {
				id: Id::from_str(&result.session_id)
					.map_err(|_| SessionError::DecodeFailed)?,
				data: HashMap::from_iter([(
					SESSION_USER_KEY.to_string(),
					result.user_id.into(),
				)]),
				expiry_date: OffsetDateTime::from_unix_timestamp(
					result.expiry_time.timestamp(),
				)
				.map_err(|_| SessionError::DecodeFailed)?,
			}))
		} else {
			tracing::trace!(?session_id, "No session found");
			Ok(None)
		}
	}

	#[tracing::instrument(skip(self))]
	async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
		tracing::trace!(session_id = ?session_id, "Deleting session");

		let affected_rows = session::Entity::delete_many()
			.filter(session::Column::SessionId.eq(session_id.to_string()))
			.exec(self.conn.as_ref())
			.await
			.map_err(SessionError::DbError)?
			.rows_affected;
		tracing::trace!(affected_rows, "Removed session");

		Ok(())
	}
}
