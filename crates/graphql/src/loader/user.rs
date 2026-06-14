use async_graphql::dataloader::Loader;
use models::entity::user;
use sea_orm::prelude::*;
use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};

/// Batches user-by-id lookups so that per-node resolvers (e.g. a book club
/// member's username/avatar/user, resolved once per row in a paginated list)
/// collapse into a single `WHERE id IN (...)` query instead of N round-trips.
pub struct UserLoader {
	conn: Arc<DatabaseConnection>,
}

impl UserLoader {
	pub fn new(conn: Arc<DatabaseConnection>) -> Self {
		Self { conn }
	}
}

pub type UserLoaderKey = String;

impl Loader<UserLoaderKey> for UserLoader {
	type Value = user::Model;
	type Error = Arc<sea_orm::error::DbErr>;

	async fn load(
		&self,
		keys: &[UserLoaderKey],
	) -> Result<HashMap<UserLoaderKey, Self::Value>, Self::Error> {
		let users = user::Entity::find()
			.filter(user::Column::Id.is_in(keys.to_vec()))
			.all(self.conn.as_ref())
			.await?;

		Ok(users.into_iter().map(|u| (u.id.clone(), u)).collect())
	}
}
