use std::path::Path;

use axum::{
	body::Body,
	extract::State,
	http::{HeaderMap, Request, StatusCode},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
	config::state::AppState,
	errors::{APIError, APIResult},
};

pub const FAVICON: &str = "/favicon.ico";
const ASSETS: &str = "/assets";
const DIST: &str = "/dist";

pub(crate) fn mount(app_state: AppState) -> Router<AppState> {
	let dist_path = Path::new(&app_state.config.client_dir);

	Router::new()
		.route(FAVICON, get(favicon))
		.nest_service(ASSETS, ServeDir::new(dist_path.join("assets")))
		.nest_service(DIST, ServeDir::new(dist_path))
		.fallback(spa_fallback)
}

/// Serve a root-level static file from the client dist (registerSW.js, sw.js,
/// workbox-*.js, manifest.webmanifest, ...) when one exists, otherwise return
/// the SPA shell (index.html) with a 200 so client-side routing works on a hard
/// load.
///
/// Previously the fallback served index.html for *every* unmatched path, so a
/// request for a real root file like `/registerSW.js` returned HTML and the
/// browser choked on it ("Uncaught SyntaxError: Unexpected token '<'"). Trying
/// the file first fixes that while keeping the SPA fallback's 200 status (a bare
/// `ServeDir::not_found_service` responds 404, which would regress deep links).
async fn spa_fallback(
	State(ctx): State<AppState>,
	req: Request<Body>,
) -> APIResult<Response> {
	let dist_path = Path::new(&ctx.config.client_dir);

	let file_res = ServeDir::new(dist_path)
		.try_call(req)
		.await
		.map_err(|e| APIError::InternalServerError(e.to_string()))?;

	if file_res.status() != StatusCode::NOT_FOUND {
		return Ok(file_res.into_response());
	}

	let index_res = ServeFile::new(dist_path.join("index.html"))
		.try_call(Request::new(Body::empty()))
		.await
		.map_err(|e| APIError::InternalServerError(e.to_string()))?;
	Ok(index_res.into_response())
}

pub(crate) fn relative_favicon_path() -> String {
	format!("{ASSETS}{FAVICON}")
}

// https://github.com/tokio-rs/axum/discussions/608#discussioncomment-7772294
async fn favicon(
	State(ctx): State<AppState>,
	headers: HeaderMap,
) -> APIResult<impl IntoResponse> {
	let mut req = Request::new(Body::empty());
	*req.headers_mut() = headers;
	match ServeFile::new(Path::new(&ctx.config.client_dir).join("favicon.ico"))
		.try_call(req)
		.await
	{
		Ok(res) => Ok(res),
		Err(e) => {
			tracing::error!(error = ?e, "Error serving favicon");
			Err(APIError::InternalServerError(e.to_string()))
		},
	}
}
