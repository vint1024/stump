use std::{fs::File, path::Path};

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek, Write};
use std::path::PathBuf;
use zip::{result::ZipError, write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::filesystem::{media::zip::ZipProcessor, FileError, FileParts, PathUtils};

pub fn write_into_zip<P: AsRef<Path>>(
	book_path: P,
	metadata_buf: Vec<u8>,
) -> Result<(), FileError> {
	let FileParts { extension, .. } = book_path.as_ref().file_parts();

	let metadata_enclosed_path = if extension == "epub" {
		// EpubProcessor::get_path_to_metadata()
		todo!("implement epub get_path_to_metadata")
	} else {
		ZipProcessor::get_path_to_metadata(
			book_path.as_ref().to_string_lossy().to_string().as_ref(),
		)?
		.unwrap_or_else(|| "ComicInfo.xml".to_string())
	};

	let mut buffer: Vec<u8> = Vec::new();
	let mut zip_writer = ZipWriter::new(Cursor::new(&mut buffer));

	let mut zip_archive = ZipArchive::new(File::open(book_path.as_ref())?)?;
	zip_writer.replace(
		&mut zip_archive,
		BTreeMap::from([(PathBuf::from(metadata_enclosed_path), metadata_buf)]),
	)?;
	zip_writer.finish()?;

	Ok(())
}

// ty <3 -> https://github.com/PaulmannLighting/smik-jar-tool/tree/main
pub trait UpdateMetadata {
	/// Copies the specified the files from the given [`ZipArchive`] into `self`,
	/// except for the files listed in `exclude`. The exclusion only applies to the
	/// write operation, any valid entry will still return a corresponding [`SimpleFileOptions`]
	/// keyed to its path.
	fn copy_partial<T>(
		&mut self,
		src: &mut ZipArchive<T>,
		exclude: Vec<PathBuf>,
	) -> Result<BTreeMap<PathBuf, SimpleFileOptions>, ZipError>
	where
		T: Read + Seek;

	/// Adds the given files to the [`ZipArchive`] with their respective `options`, if present.
	fn add_files(
		&mut self,
		files: BTreeMap<PathBuf, Vec<u8>>,
		options: BTreeMap<PathBuf, SimpleFileOptions>,
	) -> Result<(), FileError>;

	/// Replaces the contents of the given [`ZipArchive`] with the
	/// specified files. It will first call [`Self::copy_partial`] to fill
	/// the new archive based on the existing content.
	fn replace<T>(
		&mut self,
		src: &mut ZipArchive<T>,
		files: BTreeMap<PathBuf, Vec<u8>>,
	) -> Result<(), FileError>
	where
		T: Read + Seek,
	{
		let options = self.copy_partial(src, files.keys().cloned().collect())?;
		self.add_files(files, options)?;
		Ok(())
	}
}

impl<W> UpdateMetadata for ZipWriter<W>
where
	W: Write + Seek,
{
	fn copy_partial<T>(
		&mut self,
		src: &mut ZipArchive<T>,
		exclude: Vec<PathBuf>, // e.g., ["ComicInfo.xml"]
	) -> Result<BTreeMap<PathBuf, SimpleFileOptions>, ZipError>
	where
		T: Read + Seek,
	{
		let mut file_buffer = Vec::new();
		let mut options = BTreeMap::new();
		let files: Vec<_> = src.file_names().map(ToOwned::to_owned).collect();

		for file in files {
			let mut entry = src.by_name(&file)?;

			match entry.enclosed_name() {
				Some(path) if exclude.contains(&path) => {
					tracing::debug!(?path, "Excluding file");
					options.insert(path, entry.options());
					continue;
				},
				_ => {},
			}

			if entry.is_file() {
				tracing::debug!(name = entry.name(), "Copying file");
				file_buffer.clear();
				entry.read_to_end(&mut file_buffer)?;
				self.start_file(entry.name(), entry.options())?;
				self.write_all(&file_buffer)?;
			} else if entry.is_dir() {
				tracing::debug!(name = entry.name(), "Creating directory");
				self.add_directory(entry.name(), entry.options())?;
			} else {
				tracing::warn!(entry = entry.name(), "Skipping unsupported entry");
			}
		}

		Ok(options)
	}

	fn add_files(
		&mut self,
		files: BTreeMap<PathBuf, Vec<u8>>,
		mut options: BTreeMap<PathBuf, SimpleFileOptions>,
	) -> Result<(), FileError> {
		for (path, contents) in files {
			self.start_file_from_path(&path, options.remove(&path).unwrap_or_default())?;
			self.write_all(&contents)?;
		}

		Ok(())
	}
}

// TODO: this feels useful, maybe use it?
// https://github.com/PaulmannLighting/smik-jar-tool/blob/main/smik-jar-lib/src/by_path.rs

#[cfg(test)]
mod tests {
	// does indeed already have ComicInfo.xml
	#[tokio::test]
	async fn test_zip_replace_comic_info() {
		// note: may need to unzip book.zip to check if it has metadata. if not, copy the zip, unzip, add one, zip back up, put into repo as `book-with-comic-info.zip` or sm
		// 1. copy core/integration-tests/data/book.zip to /tmp or something (via tempfile dev dependency, see other examples in repo)
		// 2. create generic XML, convert to bytes
		// 3. pass path to tmp file and bytes to write_into_zip
		// 4. unzip tmp file OR get metadata raw from ZipProcessor
		// 5. assert metadata is expected
		//
		// see also: get_test_zip_path, test_process
	}

	// doesn't already have ComicInfo.xml
	#[tokio::test]
	async fn test_zip_insert_comic_info() {
		// same exact steps as test_zip_replace_comic_info but with a book zip file which doesn't have metadata
	}

	// TODO: test_zip_replace_opf
}
