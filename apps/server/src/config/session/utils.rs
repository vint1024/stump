use std::sync::Arc;
use stump_core::Ctx;

use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};

use super::StumpSessionStore;

pub const SESSION_USER_KEY: &str = "user_id";
pub const SESSION_NAME: &str = "stump_session";
pub const SESSION_PATH: &str = "/";

pub fn get_session_layer(ctx: Arc<Ctx>) -> SessionManagerLayer<StumpSessionStore> {
	let store = StumpSessionStore::new(ctx.conn.clone(), ctx.config.clone());

	let cleanup_interval = ctx.config.expired_session_cleanup_interval;
	if cleanup_interval > 0 {
		tracing::trace!(
			cleanup_interval = cleanup_interval,
			"Spawning session expiry cleanup task"
		);
		tokio::task::spawn(store.clone().continuously_delete_expired(
			tokio::time::Duration::from_secs(cleanup_interval),
		));
	} else {
		tracing::debug!("expired_session_cleanup_interval is set to 0. Session expiry cleanup is disabled");
	}

	// TODO: This configuration won't work for Tauri Windows app, it requires SameSite::None and Secure=true... Linux and macOS work fine.
	// The cookie is a session cookie (no Max-Age): tower-sessions only re-emits the
	// Set-Cookie when the session is modified, which (with our read-only auth
	// middleware) happens just once, at login — so an `OnInactivity` Max-Age would
	// freeze in the browser at login+ttl and log the user out on a fixed schedule
	// regardless of activity. Instead the server-side `expiry_time` is the source of
	// truth: `StumpSessionStore::load` gates on it and the auth middleware slides it
	// forward on activity (`touch_expiry`), giving a real sliding-inactivity window.
	SessionManagerLayer::new(store)
		.with_name(SESSION_NAME)
		.with_expiry(Expiry::OnSessionEnd)
		.with_path(SESSION_PATH.to_string())
		.with_same_site(SameSite::Lax)
		.with_secure(false)
}

/// Returns a tuple with the Set-Cookie header name and value to delete the session cookie.
/// To do this, we'll just set the cookie on the same name, path and domain, but with an
/// Expires value in the past. This *should* hopefully trigger the client to delete the cookie.
pub fn delete_cookie_header() -> (String, String) {
	(
		"Set-Cookie".to_string(),
		format!(
			"{}={}; HttpOnly; SameSite=Lax; Path={}; Domain={}; Expires={}; Max-Age=0",
			SESSION_NAME, "", SESSION_PATH, "", "Thu, 01 Jan 1970 00:00:00 GMT"
		),
	)
}
