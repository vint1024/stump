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
		"rar" | "cbr" => todo!(),
		ext => Err(FileError::UnsupportedFileType(format!(
			"File with extension {ext} is not supported for this operation"
		))),
	}
}
