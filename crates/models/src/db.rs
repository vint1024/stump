//! SQLite connection helpers shared by the server and tests.
//!
//! SQLite's built-in `lower()` and `LIKE` only case-fold ASCII, so values like
//! "Эксмо" and "эксмо" never match. We register a custom deterministic
//! function `ulower(X)` (Unicode-aware lowercase, backed by Rust's
//! `str::to_lowercase`) on every pooled connection and use it wherever
//! content-rule matching needs case-insensitive comparisons. Every code path
//! that may run such queries must connect through [`connect_sqlite`].

use std::{
	os::raw::{c_char, c_int},
	str::FromStr,
};

use sea_orm::{
	sqlx::{
		self,
		sqlite::{SqliteConnectOptions, SqlitePoolOptions},
	},
	DatabaseConnection, DbErr, RuntimeErr, SqlxSqliteConnector,
};

/// `lower()` folds ASCII only; `ulower()` is registered by [`connect_sqlite`]
/// and folds the full Unicode range. Referenced from query builders so the
/// function name lives in one place
pub const UNICODE_LOWER_FN: &str = "ulower";

/// Connect to a SQLite database with Stump's custom SQL functions registered
/// on every pooled connection. `url` is a regular sqlx URL, e.g.
/// `sqlite://path/stump.db?mode=rwc` or `sqlite::memory:`
pub async fn connect_sqlite(url: &str) -> Result<DatabaseConnection, DbErr> {
	let options = SqliteConnectOptions::from_str(url)
		.map_err(|error| DbErr::Conn(RuntimeErr::SqlxError(error)))?;
	let pool = SqlitePoolOptions::new()
		.after_connect(|conn, _meta| {
			Box::pin(async move { register_unicode_functions(conn).await })
		})
		.connect_with(options)
		.await
		.map_err(|error| DbErr::Conn(RuntimeErr::SqlxError(error)))?;
	Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

/// Register `ulower` on a raw sqlx connection. Public so non-pool consumers
/// (tests, one-off connections) can opt in as well
pub async fn register_unicode_functions(
	conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
	let mut handle = conn.lock_handle().await?;
	let raw = handle.as_raw_handle();
	let rc = unsafe {
		libsqlite3_sys::sqlite3_create_function_v2(
			raw.as_ptr(),
			c"ulower".as_ptr(),
			1,
			libsqlite3_sys::SQLITE_UTF8 | libsqlite3_sys::SQLITE_DETERMINISTIC,
			std::ptr::null_mut(),
			Some(ulower),
			None,
			None,
			None,
		)
	};
	if rc != libsqlite3_sys::SQLITE_OK {
		return Err(sqlx::Error::Protocol(format!(
			"failed to register the ulower() SQLite function (rc={rc})"
		)));
	}
	Ok(())
}

/// `SQLITE_TRANSIENT` is a C macro ((void(*)(void*))-1) that bindgen cannot
/// express, so libsqlite3-sys does not export it. It tells SQLite to copy the
/// result buffer before the callback returns
fn sqlite_transient() -> libsqlite3_sys::sqlite3_destructor_type {
	#[allow(clippy::transmute_null_to_fn)]
	Some(unsafe {
		std::mem::transmute::<isize, unsafe extern "C" fn(*mut std::ffi::c_void)>(
			-1_isize,
		)
	})
}

/// The C callback behind `ulower(X)`: NULL for NULL, Unicode lowercase
/// otherwise. Invalid UTF-8 is replaced rather than erroring — metadata
/// columns are user/file controlled and must never make a query fail
unsafe extern "C" fn ulower(
	ctx: *mut libsqlite3_sys::sqlite3_context,
	argc: c_int,
	argv: *mut *mut libsqlite3_sys::sqlite3_value,
) {
	use libsqlite3_sys as ffi;

	if argc != 1 || argv.is_null() {
		ffi::sqlite3_result_null(ctx);
		return;
	}
	let value = *argv;
	if value.is_null() || ffi::sqlite3_value_type(value) == ffi::SQLITE_NULL {
		ffi::sqlite3_result_null(ctx);
		return;
	}
	let text = ffi::sqlite3_value_text(value);
	if text.is_null() {
		ffi::sqlite3_result_null(ctx);
		return;
	}
	let len = ffi::sqlite3_value_bytes(value);
	let bytes = std::slice::from_raw_parts(text.cast::<u8>(), len as usize);
	let lowered = String::from_utf8_lossy(bytes).to_lowercase();
	let Ok(lowered_len) = c_int::try_from(lowered.len()) else {
		ffi::sqlite3_result_null(ctx);
		return;
	};
	ffi::sqlite3_result_text(
		ctx,
		lowered.as_ptr().cast::<c_char>(),
		lowered_len,
		sqlite_transient(),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use sea_orm::{ConnectionTrait, Statement};

	#[test]
	fn ulower_folds_unicode() {
		tokio_test::block_on(async {
			let conn = connect_sqlite("sqlite::memory:").await.expect("connect");
			let row = conn
				.query_one(Statement::from_string(
					sea_orm::DatabaseBackend::Sqlite,
					"SELECT ulower('ЭкСмО AbC') AS v, ulower(NULL) AS n".to_string(),
				))
				.await
				.expect("query")
				.expect("row");
			let v: String = row.try_get("", "v").expect("v");
			assert_eq!(v, "эксмо abc");
			let n: Option<String> = row.try_get("", "n").expect("n");
			assert_eq!(n, None);
		});
	}
}
