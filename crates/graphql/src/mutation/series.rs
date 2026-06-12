use async_graphql::{Context, Object, Result, ID};
use chrono::Utc;
use models::{
	entity::{
		favorite_series, finished_reading_session, library, library_config, media,
		reading_session, series, series_merge, user::AuthUser,
	},
	shared::enums::{FileStatus, UserPermission},
};
use sea_orm::{
	prelude::*,
	sea_query::{OnConflict, Query},
	ActiveValue::Set,
	QuerySelect, TransactionTrait,
};
use stump_core::job::stump_job::StumpJob;
use stump_core::{
	database::SQLITE_BIND_LIMIT,
	filesystem::{
		image::{
			generate_book_thumbnail, remove_thumbnails, GenerateThumbnailOptions,
			ThumbnailGenerationJobParams,
		},
		media::analysis::{AnalysisJobConfig, MediaAnalysisJobScope},
	},
};

use crate::{
	data::{AuthContext, CoreContext},
	guard::PermissionGuard,
	input::thumbnail::UpdateThumbnailInput,
	object::series::Series,
};

#[derive(Default)]
pub struct SeriesMutation;

#[Object]
impl SeriesMutation {
	/// Permanently delete a single series and all of its books from the
	/// database. Files on disk are NOT touched. This is the per-series
	/// counterpart to "Clean Library" — handy for removing a series whose
	/// folder is gone (status "Missing") without wiping the whole library.
	#[graphql(guard = "PermissionGuard::one(UserPermission::ManageLibrary)")]
	async fn delete_series(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		// Access control: the user must be able to see the series
		let series =
			series::Entity::find_series_ident_for_user_and_id(user, id.to_string())
				.into_model::<series::SeriesIdentSelect>()
				.one(conn)
				.await?
				.ok_or("Series not found")?;

		let txn = conn.begin().await?;
		// Books first — FK cascades take care of metadata, tags, reading
		// sessions, bookmarks, etc. The files on disk are left alone.
		let deleted_media_ids = media::Entity::delete_many()
			.filter(media::Column::SeriesId.eq(series.id.clone()))
			.exec_with_returning(&txn)
			.await?
			.into_iter()
			.map(|m| m.id)
			.collect::<Vec<_>>();
		// Drop any merge-map entries that fed books into this series, so a
		// later scan doesn't try to route them to a now-deleted target.
		series_merge::Entity::delete_many()
			.filter(series_merge::Column::TargetSeriesId.eq(series.id.clone()))
			.exec(&txn)
			.await?;
		series::Entity::delete_by_id(series.id.clone())
			.exec(&txn)
			.await?;
		txn.commit().await?;

		// Best-effort thumbnail cleanup, mirroring clean_library so deleting a
		// series doesn't leave orphaned cached thumbnails on disk.
		let thumbnails_dir = core.config.get_thumbnails_dir();
		if !deleted_media_ids.is_empty() {
			if let Err(error) =
				remove_thumbnails(&deleted_media_ids, &thumbnails_dir).await
			{
				tracing::error!(
					?error,
					"Failed to remove thumbnails for deleted series media"
				);
			}
		}
		if let Err(error) = remove_thumbnails(&[series.id.clone()], &thumbnails_dir).await
		{
			tracing::error!(?error, "Failed to remove thumbnail for deleted series");
		}

		tracing::debug!(series_id = %series.id, "Deleted series");
		Ok(true)
	}

	#[graphql(guard = "PermissionGuard::one(UserPermission::ManageLibrary)")]
	async fn analyze_series(
		&self,
		ctx: &Context<'_>,
		id: ID,
		#[graphql(default = false)] force_reanalysis: bool,
	) -> Result<bool> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		let model =
			series::Entity::find_series_ident_for_user_and_id(user, id.to_string())
				.into_model::<series::SeriesIdentSelect>()
				.one(conn)
				.await?
				.ok_or("Series not found")?;

		core.enqueue(StumpJob::analyze_media(AnalysisJobConfig {
			force_reanalysis,
			scope: MediaAnalysisJobScope::Series(model.id),
		}))
		.await?;

		Ok(true)
	}

	async fn favorite_series(
		&self,
		ctx: &Context<'_>,
		id: ID,
		is_favorite: bool,
	) -> Result<Series> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		let model = series::ModelWithMetadata::find_for_user(user)
			.filter(
				series::Column::Id
					.eq(id.to_string())
					.and(series::Column::DeletedAt.is_null()),
			)
			.into_model::<series::ModelWithMetadata>()
			.one(conn)
			.await?
			.ok_or("Series not found")?;

		if is_favorite {
			let last_insert_id =
				favorite_series::Entity::insert(favorite_series::ActiveModel {
					user_id: Set(user.id.clone()),
					series_id: Set(model.series.id.clone()),
					favorited_at: Set(DateTimeWithTimeZone::from(Utc::now())),
				})
				.on_conflict(OnConflict::new().do_nothing().to_owned())
				.exec(core.conn.as_ref())
				.await?
				.last_insert_id;
			tracing::debug!(?last_insert_id, "Added favorite series");
		} else {
			let affected_rows =
				favorite_series::Entity::delete_many()
					.filter(favorite_series::Column::UserId.eq(user.id.clone()).and(
						favorite_series::Column::SeriesId.eq(model.series.id.clone()),
					))
					.exec(core.conn.as_ref())
					.await?
					.rows_affected;
			tracing::debug!(?affected_rows, "Removed favorite series");
		}

		Ok(model.into())
	}

	/// Update the thumbnail for a series. This will replace the existing thumbnail with the the one
	/// associated with the provided input (book). If the book does not have a thumbnail, one
	/// will be generated based on the library's thumbnail configuration.
	#[graphql(guard = "PermissionGuard::one(UserPermission::EditThumbnails)")]
	async fn update_series_thumbnail(
		&self,
		ctx: &Context<'_>,
		id: ID,
		input: UpdateThumbnailInput,
	) -> Result<Series> {
		let core = ctx.data::<CoreContext>()?;
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;

		let series = series::ModelWithMetadata::find_for_user(user)
			.filter(series::Column::Id.eq(id.to_string()))
			.into_model::<series::ModelWithMetadata>()
			.one(core.conn.as_ref())
			.await?
			.ok_or("Series not found")?;
		let series_id = series.series.id.clone();

		let (_library, config) = library::Entity::find_for_user(user)
			.filter(
				library::Column::Id.in_subquery(
					Query::select()
						.column(series::Column::LibraryId)
						.from(series::Entity)
						.and_where(series::Column::Id.eq(series_id))
						.to_owned(),
				),
			)
			.find_also_related(library_config::Entity)
			.one(core.conn.as_ref())
			.await?
			.ok_or("Associated library for series not found")?;

		let book = media::Entity::find_for_user(user)
			.filter(media::Column::Id.eq(input.media_id.to_string()))
			.one(core.conn.as_ref())
			.await?
			.ok_or("Media not found")?;

		let page = input.params.page();

		if book.extension == "epub" && page > 1 {
			return Err("Cannot set thumbnail from EPUB chapter".into());
		}

		let image_options = config
			.ok_or("Library config not found")?
			.thumbnail_config
			.unwrap_or_default()
			.with_page(page);

		let (_, path_buf, _) = generate_book_thumbnail(
			&book.clone().into(),
			core.conn.as_ref(),
			GenerateThumbnailOptions {
				image_options,
				core_config: core.config.as_ref().clone(),
				force_regen: true,
				filename: Some(id.to_string()),
			},
		)
		.await?;
		tracing::debug!(path = ?path_buf, "Generated series thumbnail");

		Ok(series.into())
	}

	/// Toggle the completion status of a series. If the series is marked as completed, all books
	/// in the series will also be marked as completed, and vice versa for marking as not completed.
	/// This is considered a dangerous operation since it can modify all your read progression related
	/// to a single series all at once. Please use with caution.
	async fn toggle_series_completion(
		&self,
		ctx: &Context<'_>,
		id: ID,
		is_completed: bool,
	) -> Result<Series> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;

		let series = series::ModelWithMetadata::find_for_user(user)
			.filter(series::Column::Id.eq(id.to_string()))
			.into_model::<series::ModelWithMetadata>()
			.one(core.conn.as_ref())
			.await?
			.ok_or("Series not found")?;

		if is_completed {
			set_series_completed(core, user, &series).await?;
		} else {
			unset_series_completed(core, user, &series).await?;
		}

		Ok(series.into())
	}

	#[graphql(guard = "PermissionGuard::one(UserPermission::ScanLibrary)")]
	async fn scan_series(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		let model =
			series::Entity::find_series_ident_for_user_and_id(user, id.to_string())
				.into_model::<series::SeriesIdentSelect>()
				.one(conn)
				.await?
				.ok_or("Series not found")?;

		core.enqueue(StumpJob::series_scan(model.id, model.path, None))
			.await?;

		Ok(true)
	}

	/// Enqueue a job which (re)generates the thumbnail for a single series from
	/// its first book. The library-level job covers series too, but only as part
	/// of a full sweep — this gives the series page its own regenerate action
	#[graphql(guard = "PermissionGuard::one(UserPermission::EditThumbnails)")]
	async fn generate_series_thumbnail(
		&self,
		ctx: &Context<'_>,
		id: ID,
		#[graphql(default = true)] force_regenerate: bool,
	) -> Result<bool> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		let model = series::Entity::find_for_user(user)
			.filter(series::Column::Id.eq(id.to_string()))
			.one(conn)
			.await?
			.ok_or("Series not found")?;

		let library_id = model.library_id.clone().ok_or("Series has no library")?;
		let config = library::Entity::find()
			.filter(library::Column::Id.eq(library_id))
			.find_also_related(library_config::Entity)
			.one(conn)
			.await?
			.and_then(|(_, config)| config)
			.ok_or("Library config not found")?;

		core.enqueue(StumpJob::thumbnail_generation(
			config.thumbnail_config.unwrap_or_default(),
			ThumbnailGenerationJobParams::series(vec![model.id], force_regenerate),
		))
		.await?;

		Ok(true)
	}

	/// Merge one or more series into a target series: their books move to the
	/// target and the source series are removed. A persistent merge map keeps
	/// the scanner from re-creating the source folders as series, and allows
	/// the merge to be undone later. All series must be in the same library
	#[graphql(guard = "PermissionGuard::one(UserPermission::ManageLibrary)")]
	async fn merge_series(
		&self,
		ctx: &Context<'_>,
		target_id: ID,
		source_ids: Vec<ID>,
	) -> Result<Series> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		if source_ids.is_empty() {
			return Err("At least one source series is required".into());
		}
		let source_ids = source_ids
			.iter()
			.map(|id| id.to_string())
			.collect::<Vec<_>>();
		if source_ids.contains(&target_id.to_string()) {
			return Err("A series cannot be merged into itself".into());
		}

		let target = series::Entity::find_for_user(user)
			.filter(series::Column::Id.eq(target_id.to_string()))
			.one(conn)
			.await?
			.ok_or("Target series not found")?;

		let sources = series::Entity::find_for_user(user)
			.filter(series::Column::Id.is_in(source_ids.clone()))
			.all(conn)
			.await?;
		if sources.len() != source_ids.len() {
			return Err("One or more source series were not found".into());
		}
		if sources
			.iter()
			.any(|source| source.library_id != target.library_id)
		{
			return Err("Series can only be merged within the same library".into());
		}

		let txn = conn.begin().await?;

		for source in &sources {
			media::Entity::update_many()
				.col_expr(media::Column::SeriesId, Expr::value(target.id.clone()))
				.filter(media::Column::SeriesId.eq(source.id.clone()))
				.exec(&txn)
				.await?;

			// If the source was itself a merge target, re-point its absorbed
			// folders at the new target so chained merges stay resolvable
			series_merge::Entity::update_many()
				.col_expr(
					series_merge::Column::TargetSeriesId,
					Expr::value(target.id.clone()),
				)
				.filter(series_merge::Column::TargetSeriesId.eq(source.id.clone()))
				.exec(&txn)
				.await?;

			series_merge::ActiveModel {
				target_series_id: Set(target.id.clone()),
				source_path: Set(source.path.clone()),
				source_name: Set(source.name.clone()),
				..Default::default()
			}
			.insert(&txn)
			.await?;

			series::Entity::delete_by_id(source.id.clone())
				.exec(&txn)
				.await?;
		}

		txn.commit().await?;

		tracing::debug!(
			target_id = %target.id,
			source_count = sources.len(),
			"Merged series"
		);

		Ok(series::ModelWithMetadata {
			series: target,
			metadata: None,
		}
		.into())
	}

	/// Undo every merge into the given series: each absorbed folder becomes its
	/// own series again (with its original name) and its books move back
	#[graphql(guard = "PermissionGuard::one(UserPermission::ManageLibrary)")]
	async fn unmerge_series(&self, ctx: &Context<'_>, id: ID) -> Result<Series> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let core = ctx.data::<CoreContext>()?;
		let conn = core.conn.as_ref();

		let target = series::Entity::find_for_user(user)
			.filter(series::Column::Id.eq(id.to_string()))
			.one(conn)
			.await?
			.ok_or("Series not found")?;

		let merges = series_merge::Entity::find()
			.filter(series_merge::Column::TargetSeriesId.eq(target.id.clone()))
			.all(conn)
			.await?;
		if merges.is_empty() {
			return Err("This series has no merged folders to restore".into());
		}

		let txn = conn.begin().await?;

		for merge in &merges {
			let restored = series::ActiveModel {
				id: Set(Uuid::new_v4().to_string()),
				name: Set(merge.source_name.clone()),
				path: Set(merge.source_path.clone()),
				status: Set(FileStatus::Ready),
				library_id: Set(target.library_id.clone()),
				created_at: Set(Utc::now().into()),
				..Default::default()
			}
			.insert(&txn)
			.await?;

			// Books that live under the restored folder move back to it. Build
			// the LIKE prefix with escaping — folder names routinely contain '_'
			// (a LIKE wildcard), which would otherwise pull in sibling folders.
			let mut source_prefix = merge.source_path.clone();
			if !source_prefix.ends_with('/') && !source_prefix.ends_with('\\') {
				source_prefix.push(if source_prefix.contains('/') {
					'/'
				} else {
					'\\'
				});
			}
			let escaped_prefix = source_prefix
				.replace('\\', "\\\\")
				.replace('%', "\\%")
				.replace('_', "\\_");
			media::Entity::update_many()
				.col_expr(media::Column::SeriesId, Expr::value(restored.id.clone()))
				.filter(media::Column::SeriesId.eq(target.id.clone()))
				.filter(Expr::cust_with_values(
					"\"media\".\"path\" LIKE ? ESCAPE '\\'",
					[format!("{escaped_prefix}%")],
				))
				.exec(&txn)
				.await?;
		}

		series_merge::Entity::delete_many()
			.filter(series_merge::Column::TargetSeriesId.eq(target.id.clone()))
			.exec(&txn)
			.await?;

		txn.commit().await?;

		tracing::debug!(
			target_id = %target.id,
			restored = merges.len(),
			"Un-merged series"
		);

		Ok(series::ModelWithMetadata {
			series: target,
			metadata: None,
		}
		.into())
	}
}

async fn set_series_completed(
	core: &CoreContext,
	user: &AuthUser,
	series: &series::ModelWithMetadata,
) -> Result<()> {
	let tx = core.conn.begin().await?;

	let book_ids_without_completion = media::Entity::find_for_user(user)
		.select_only()
		.column(media::Column::Id)
		.filter(
			media::Column::SeriesId.eq(series.series.id.clone()).and(
				// to get books WITHOUT completion means they do not have a finished_reading_session
				media::Column::Id.not_in_subquery(
					Query::select()
						.column(finished_reading_session::Column::MediaId)
						.from(finished_reading_session::Entity)
						.and_where(
							finished_reading_session::Column::UserId.eq(user.id.clone()),
						)
						.to_owned(),
				),
			),
		)
		.into_tuple::<String>()
		.all(&tx)
		.await?;

	tracing::debug!(
		count = book_ids_without_completion.len(),
		"Fetched unread books within series"
	);

	let deleted_sessions = reading_session::Entity::delete_many()
		.filter(
			reading_session::Column::UserId.eq(user.id.clone()).and(
				reading_session::Column::MediaId.in_subquery(
					Query::select()
						.column(media::Column::Id)
						.from(media::Entity)
						.and_where(media::Column::SeriesId.eq(series.series.id.clone()))
						.to_owned(),
				),
			),
		)
		// with returning so we can reuse the deleted session info when creating the
		// completion records (e.g., elapsed time etc)
		.exec_with_returning(&tx)
		.await?;
	tracing::debug!(
		count = deleted_sessions.len(),
		"Deleted active reading sessions for series"
	);

	// this just makes it more efficient to pull the previous one before creating
	// the completion record
	let session_map = deleted_sessions
		.into_iter()
		.map(|s| (s.media_id.clone(), s))
		.collect::<std::collections::HashMap<_, _>>();

	let now = DateTimeWithTimeZone::from(Utc::now());
	let finished_sessions_chunks = book_ids_without_completion
		.into_iter()
		.map(|media_id| {
			let prior = session_map.get(&media_id);
			finished_reading_session::ActiveModel {
				user_id: Set(user.id.clone()),
				media_id: Set(media_id),
				started_at: Set(prior.map(|s| s.started_at).unwrap_or(now)),
				completed_at: Set(now),
				elapsed_seconds: Set(prior.and_then(|s| s.elapsed_seconds)),
				device_id: Set(prior.and_then(|s| s.device_id.clone())),
				..Default::default()
			}
		})
		.collect::<Vec<finished_reading_session::ActiveModel>>();
	if !finished_sessions_chunks.is_empty() {
		for chunk in finished_sessions_chunks.chunks(SQLITE_BIND_LIMIT) {
			let count = chunk.len();
			let _ = finished_reading_session::Entity::insert_many(chunk.to_vec())
				.exec(&tx)
				.await?;
			tracing::debug!(count, "Inserted finished reading sessions for series");
		}
	} else {
		tracing::debug!("No books to mark as finished in series");
	}

	tx.commit().await?;

	Ok(())
}

async fn unset_series_completed(
	core: &CoreContext,
	user: &AuthUser,
	series: &series::ModelWithMetadata,
) -> Result<()> {
	let tx = core.conn.begin().await?;

	let affected_rows = finished_reading_session::Entity::delete_many()
		.filter(
			finished_reading_session::Column::UserId
				.eq(user.id.clone())
				.and(
					finished_reading_session::Column::MediaId.in_subquery(
						Query::select()
							.column(media::Column::Id)
							.from(media::Entity)
							.and_where(
								media::Column::SeriesId.eq(series.series.id.clone()),
							)
							.to_owned(),
					),
				),
		)
		.exec(&tx)
		.await?
		.rows_affected;

	tracing::debug!(
		?affected_rows,
		"Removed finished reading sessions for series"
	);

	tx.commit().await?;

	Ok(())
}
