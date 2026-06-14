use axum::{
	extract::{Path, State},
	http::HeaderMap,
	middleware,
	response::IntoResponse,
	routing::{get, post},
	Extension, Json, Router,
};
use base64::Engine;
use graphql::data::AuthContext;
use models::{
	entity::{device_public_key, library, library_config, media, series, user::AuthUser},
	shared::{enums::UserPermission, image_processor_options::SupportedImageFormat},
};
use sea_orm::{
	prelude::*,
	sea_query::{OnConflict, Query},
	QuerySelect,
};
use stump_core::{
	config::StumpConfig,
	filesystem::{
		get_saved_thumbnail, get_thumbnail, media::get_page_async, ContentType, FileError,
	},
	Ctx,
};

use crate::{
	config::state::AppState,
	errors::{APIError, APIResult},
	middleware::auth::auth_middleware,
	utils::{
		http::ImageResponse,
		offline_crypto::{validate_device_key, wrap_for_device},
		serve_media,
	},
};

pub(crate) fn mount(app_state: AppState) -> Router<AppState> {
	Router::new()
		.nest(
			"/media/{id}",
			Router::new()
				.route("/thumbnail", get(get_media_thumbnail_handler))
				.route("/page/{page}", get(get_media_page))
				.route("/file", get(get_media_file))
				.route("/offline", post(get_media_offline)),
		)
		.route("/media/device", post(register_device))
		.layer(middleware::from_fn_with_state(app_state, auth_middleware))
}

/// Download the file associated with the media.
pub(crate) async fn get_media_file(
	Path(id): Path<String>,
	State(ctx): State<AppState>,
	Extension(req): Extension<AuthContext>,
	headers: HeaderMap,
) -> APIResult<impl IntoResponse> {
	serve_media::serve_media_file(req, headers, ctx.conn.as_ref(), id).await
}

pub(crate) async fn get_media_thumbnail(
	book: &media::MediaThumbSelect,
	image_format: Option<SupportedImageFormat>,
	config: &StumpConfig,
) -> APIResult<(ContentType, Vec<u8>)> {
	// Note: This doesn't hard-fail because if the saved thumbnail is missing or corrupt, we want
	// to just pull something else instead of erroring out entirely.
	if let Some(path) = &book.thumbnail_path {
		match get_saved_thumbnail(std::path::Path::new(path)).await {
			Ok(result) => return Ok(result),
			Err(_) => {
				tracing::warn!(path = ?path, "Failed to get saved thumbnail");
			},
		}
	}

	let generated_thumb =
		get_thumbnail(config.get_thumbnails_dir(), &book.id, image_format).await?;

	let adjusted_config = StumpConfig {
		pdf_prerender_range: 0, // Disable PDF prerendering for thumbnails since we only need the first page
		..config.clone()
	};

	if let Some((content_type, bytes)) = generated_thumb {
		Ok((content_type, bytes))
	} else {
		Ok(get_page_async(&book.path, 1, &adjusted_config).await?)
	}
}

pub(crate) async fn get_media_thumbnail_by_id(
	ctx: &Ctx,
	user: &AuthUser,
	book_id: String,
) -> APIResult<ImageResponse> {
	let book = media::Entity::find_for_user(user)
		.columns(media::MediaThumbSelect::columns())
		.filter(media::Column::Id.eq(book_id))
		.into_model::<media::MediaThumbSelect>()
		.one(ctx.conn.as_ref())
		.await?
		.ok_or(APIError::NotFound("Book not found".to_string()))?;

	// Note: This doesn't hard-fail because if the saved thumbnail is missing or corrupt, we want
	// to just pull something else instead of erroring out entirely.
	if let Some(path) = &book.thumbnail_path {
		match get_saved_thumbnail(std::path::Path::new(path)).await {
			Ok(result) => return Ok(result.into()),
			Err(_) => {
				tracing::warn!(path = ?path, "Failed to get saved thumbnail");
			},
		}
	}

	let library_config = library_config::Entity::find()
		.filter(
			library_config::Column::LibraryId.in_subquery(
				Query::select()
					.column(library::Column::Id)
					.from(library::Entity)
					.and_where(
						library::Column::Id.in_subquery(
							Query::select()
								.column(series::Column::LibraryId)
								.from(series::Entity)
								.and_where(series::Column::Id.eq(book.series_id.clone()))
								.to_owned(),
						),
					)
					.to_owned(),
			),
		)
		.one(ctx.conn.as_ref())
		.await?;
	let image_format = library_config.and_then(|o| o.thumbnail_config.map(|c| c.format));

	get_media_thumbnail(&book, image_format, ctx.config.as_ref())
		.await
		.map(ImageResponse::from)
}

pub(crate) async fn get_media_thumbnail_handler(
	Path(id): Path<String>,
	State(ctx): State<AppState>,
	Extension(req): Extension<AuthContext>,
) -> APIResult<ImageResponse> {
	get_media_thumbnail_by_id(&ctx, &req.user(), id).await
}

async fn get_media_page(
	Path((id, page)): Path<(String, u32)>,
	State(ctx): State<AppState>,
	Extension(req): Extension<AuthContext>,
) -> APIResult<ImageResponse> {
	let book = media::Entity::find_for_user(&req.user())
		.filter(media::Column::Id.eq(id.clone()))
		.into_model::<media::MediaIdentSelect>()
		.one(ctx.conn.as_ref())
		.await?
		.ok_or(APIError::NotFound("Book not found".to_string()))?;

	let content =
		match get_page_async(&book.path, page.try_into()?, ctx.config.as_ref()).await {
			Ok(result) => result,
			Err(e) => {
				if matches!(e, FileError::NoImageError) {
					return Err(APIError::NotFound("Page not found".to_string()));
				}
				return Err(APIError::InternalServerError(e.to_string()));
			},
		};

	Ok(ImageResponse::from(content))
}

// ── E3 offline reading (per-device encrypted delivery) ──────────────────────────

/// Max accepted length of a client-supplied `device_id` (defensive bound).
const MAX_DEVICE_ID_LEN: usize = 256;
/// Max registered devices per user, to bound row growth from a misbehaving or
/// hostile client. Re-registering an existing device does not count against this.
const MAX_DEVICES_PER_USER: u64 = 50;
/// Max book size we will encrypt for offline delivery in a single in-memory pass.
/// The whole file is read, encrypted, and base64-encoded in RAM (~3.3× peak), so
/// we cap it to avoid OOM/DoS. 1 GiB covers any realistic book.
const MAX_OFFLINE_BOOK_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(serde::Deserialize)]
pub(crate) struct RegisterDeviceInput {
	device_id: String,
	/// The device's P-256 public key, base64 of the X9.63 uncompressed point (65 bytes).
	public_key: String,
}

/// Register (or update) a device's public key, so offline book content can be
/// wrapped to it. The private key never leaves the device's Secure Enclave.
pub(crate) async fn register_device(
	State(ctx): State<AppState>,
	Extension(req): Extension<AuthContext>,
	Json(input): Json<RegisterDeviceInput>,
) -> APIResult<impl IntoResponse> {
	let user = req.user();

	if input.device_id.is_empty() || input.device_id.len() > MAX_DEVICE_ID_LEN {
		return Err(APIError::BadRequest("Invalid device id".to_string()));
	}

	let public_key = base64::engine::general_purpose::STANDARD
		.decode(input.public_key.as_bytes())
		.map_err(|_| APIError::BadRequest("Invalid public key".to_string()))?;
	if public_key.len() != 65 {
		return Err(APIError::BadRequest(
			"Public key must be a 65-byte X9.63 point".to_string(),
		));
	}
	// Reject anything that is not a real P-256 point now (400), rather than
	// failing later when wrapping content for this device (which would 500).
	validate_device_key(&public_key).map_err(|_| {
		APIError::BadRequest("Public key is not a valid P-256 point".to_string())
	})?;

	let conn = ctx.conn.as_ref();

	// Bound the number of devices a single user may register (new devices only;
	// re-registering an existing device is an idempotent update below).
	let existing_count = device_public_key::Entity::find()
		.filter(device_public_key::Column::UserId.eq(user.id.clone()))
		.filter(device_public_key::Column::DeviceId.ne(input.device_id.clone()))
		.count(conn)
		.await?;
	if existing_count >= MAX_DEVICES_PER_USER {
		return Err(APIError::BadRequest(
			"Too many registered devices".to_string(),
		));
	}

	// Idempotent upsert keyed on the (user_id, device_id) unique index — race-safe
	// under concurrent double-registration (no check-then-act 500).
	device_public_key::Entity::insert(device_public_key::ActiveModel {
		id: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
		user_id: sea_orm::Set(user.id.clone()),
		device_id: sea_orm::Set(input.device_id),
		public_key: sea_orm::Set(public_key),
		created_at: sea_orm::Set(chrono::Utc::now().into()),
	})
	.on_conflict(
		OnConflict::columns(vec![
			device_public_key::Column::UserId,
			device_public_key::Column::DeviceId,
		])
		.update_column(device_public_key::Column::PublicKey)
		.to_owned(),
	)
	.exec(conn)
	.await?;

	Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(serde::Deserialize)]
pub(crate) struct OfflineInput {
	device_id: String,
}

#[derive(serde::Serialize)]
pub(crate) struct OfflineResponse {
	/// AES-256-GCM(content_key, book), base64.
	blob: String,
	/// Ephemeral P-256 public key (X9.63), base64.
	ephemeral_pub: String,
	/// AES-256-GCM(kek, content_key) where kek = HKDF(ECDH(device, ephemeral)), base64.
	wrapped_key: String,
}

/// Return the book encrypted for offline reading, with the content key wrapped to
/// the requesting device's registered public key. Gated by `OfflineRead`: a user
/// without `DownloadFile` can read offline but never receives the plaintext file.
pub(crate) async fn get_media_offline(
	Path(id): Path<String>,
	State(ctx): State<AppState>,
	Extension(req): Extension<AuthContext>,
	Json(input): Json<OfflineInput>,
) -> APIResult<impl IntoResponse> {
	let user = req
		.user_and_enforce_permissions(&[UserPermission::OfflineRead])
		.map_err(|_| APIError::forbidden_discreet())?;
	let conn = ctx.conn.as_ref();

	let book = media::Entity::find_for_user(&user)
		.filter(media::Column::Id.eq(id))
		.into_model::<media::MediaIdentSelect>()
		.one(conn)
		.await?
		.ok_or(APIError::NotFound("Book not found".to_string()))?;

	let device = device_public_key::Entity::find()
		.filter(device_public_key::Column::UserId.eq(user.id.clone()))
		.filter(device_public_key::Column::DeviceId.eq(input.device_id))
		.one(conn)
		.await?
		.ok_or(APIError::BadRequest("Device not registered".to_string()))?;

	// Guard memory before reading: the whole file is held in RAM and encrypted.
	let metadata = tokio::fs::metadata(&book.path).await.map_err(|e| {
		tracing::error!(error = ?e, "Failed to stat book file for offline delivery");
		APIError::NotFound("Book file not found".to_string())
	})?;
	if metadata.len() > MAX_OFFLINE_BOOK_BYTES {
		return Err(APIError::BadRequest(
			"Book is too large for offline delivery".to_string(),
		));
	}

	let plaintext = tokio::fs::read(&book.path).await.map_err(|e| {
		tracing::error!(error = ?e, "Failed to read book file for offline delivery");
		APIError::InternalServerError("Failed to read book file".to_string())
	})?;

	// AES-GCM over the whole book is CPU-bound; keep it off the async worker.
	let device_pub = device.public_key;
	let wrapped =
		tokio::task::spawn_blocking(move || wrap_for_device(&plaintext, &device_pub))
			.await
			.map_err(|e| {
				APIError::InternalServerError(format!("Encryption task failed: {e}"))
			})?
			.map_err(|_| {
				APIError::InternalServerError("Encryption failed".to_string())
			})?;

	let b64 = base64::engine::general_purpose::STANDARD;
	Ok(Json(OfflineResponse {
		blob: b64.encode(wrapped.blob),
		ephemeral_pub: b64.encode(wrapped.ephemeral_pub),
		wrapped_key: b64.encode(wrapped.wrapped_key),
	}))
}
