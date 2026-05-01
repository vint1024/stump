use std::path::Path;

use tokio::{sync::oneshot, task::spawn_blocking};

use crate::filesystem::{
	metadata::writer::zip::write_into_zip, FileError, FileParts, PathUtils,
};

mod comic_info;
mod zip;

pub async fn write_metadata<P: AsRef<Path>>(
	book_path: P,
	metadata_buf: Vec<u8>,
) -> Result<(), FileError> {
	let (tx, rx) = oneshot::channel();

	let handle = spawn_blocking({
		let book_path = book_path.as_ref().to_path_buf();

		move || {
			let send_result = tx.send(write_metadata_blocking(book_path, metadata_buf));
			tracing::trace!(
				is_err = send_result.is_err(),
				"Sending result of sync process"
			);
		}
	});

	let result = if let Ok(recv) = rx.await {
		recv?
	} else {
		handle
			.await
			.map_err(|e| FileError::UnknownError(e.to_string()))?;
		return Err(FileError::UnknownError(
			"Failed to receive successful metadata write exchange".to_string(),
		));
	};

	Ok(result)
}

fn write_metadata_blocking<P: AsRef<Path>>(
	book_path: P,
	metadata_buf: Vec<u8>,
) -> Result<(), FileError> {
	let FileParts { extension, .. } = book_path.as_ref().file_parts();

	match extension.to_lowercase().as_str() {
		"epub" | "zip" | "cbz" => write_into_zip(book_path, metadata_buf),
		ext => Err(FileError::UnsupportedFileType(format!(
			"File with extension {ext} is not supported for this operation"
		))),
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use std::{fs, path::PathBuf};

	pub fn get_test_zip_path() -> String {
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("integration-tests/data/book.zip")
			.to_string_lossy()
			.to_string()
	}

	pub fn get_test_zip_file_data() -> Vec<u8> {
		let test_zip_path = get_test_zip_path();

		fs::read(test_zip_path).expect("Failed to fetch test zip file")
	}
}
