//! Write book metadata back INTO an epub file by rewriting the `<metadata>`
//! block of its OPF package document. The rewrite is surgical: only elements
//! Stump owns a value for are replaced, everything else inside (and outside)
//! the metadata block is preserved as-is. The archive is rebuilt next to the
//! original and atomically renamed over it; an optional `.bak` copy of the
//! original can be kept.

use std::{
	fs,
	io::{Read, Write},
	path::{Path, PathBuf},
};

use models::entity::media_metadata;
use quick_xml::{
	events::{BytesEnd, BytesStart, BytesText, Event},
	Reader, Writer,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::filesystem::error::FileError;

/// The set of values Stump writes into an OPF. Built from the book's stored
/// metadata; `None` fields leave whatever the file already has untouched
#[derive(Debug, Default, Clone)]
pub struct OpfWriteback {
	pub title: Option<String>,
	/// dc:creator, one element per writer
	pub creators: Vec<String>,
	pub publisher: Option<String>,
	pub language: Option<String>,
	/// dc:description
	pub summary: Option<String>,
	/// dc:subject, one element per genre
	pub subjects: Vec<String>,
	/// dc:date (YYYY or YYYY-MM-DD when month/day are known)
	pub date: Option<String>,
	/// calibre-style series name + index meta tags
	pub series: Option<String>,
	pub series_index: Option<String>,
}

fn split_list(value: &Option<String>) -> Vec<String> {
	value
		.as_deref()
		.unwrap_or_default()
		.split(',')
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.map(ToString::to_string)
		.collect()
}

impl From<&media_metadata::Model> for OpfWriteback {
	fn from(metadata: &media_metadata::Model) -> Self {
		let date = metadata
			.year
			.map(|year| match (metadata.month, metadata.day) {
				(Some(month), Some(day)) => format!("{year:04}-{month:02}-{day:02}"),
				(Some(month), None) => format!("{year:04}-{month:02}"),
				_ => format!("{year:04}"),
			});

		Self {
			title: metadata.title.clone(),
			creators: split_list(&metadata.writers),
			publisher: metadata.publisher.clone(),
			language: metadata.language.clone(),
			summary: metadata.summary.clone(),
			subjects: split_list(&metadata.genres),
			date,
			series: metadata.series.clone(),
			series_index: metadata.number.map(|number| number.to_string()),
		}
	}
}

impl OpfWriteback {
	pub fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.creators.is_empty()
			&& self.publisher.is_none()
			&& self.language.is_none()
			&& self.summary.is_none()
			&& self.subjects.is_empty()
			&& self.date.is_none()
			&& self.series.is_none()
			&& self.series_index.is_none()
	}
}

/// Find the OPF package document path inside the archive by parsing
/// META-INF/container.xml
fn locate_opf_path(archive: &mut ZipArchive<fs::File>) -> Result<String, FileError> {
	let mut container_xml = String::new();
	archive
		.by_name("META-INF/container.xml")
		.map_err(|e| FileError::EpubReadError(e.to_string()))?
		.read_to_string(&mut container_xml)?;

	let mut reader = Reader::from_str(&container_xml);
	let mut buf = Vec::new();
	loop {
		match reader
			.read_event_into(&mut buf)
			.map_err(|e| FileError::EpubReadError(e.to_string()))?
		{
			Event::Start(ref tag) | Event::Empty(ref tag)
				if tag.local_name().as_ref() == b"rootfile" =>
			{
				for attribute in tag.attributes().flatten() {
					if attribute.key.local_name().as_ref() == b"full-path" {
						return Ok(String::from_utf8_lossy(&attribute.value).to_string());
					}
				}
			},
			Event::Eof => break,
			_ => {},
		}
		buf.clear();
	}

	Err(FileError::EpubReadError(
		"No rootfile entry in container.xml".to_string(),
	))
}

/// Whether the writeback set replaces this element of the metadata block
fn is_replaced_element(writeback: &OpfWriteback, local_name: &[u8]) -> bool {
	match local_name {
		b"title" => writeback.title.is_some(),
		b"creator" => !writeback.creators.is_empty(),
		b"publisher" => writeback.publisher.is_some(),
		b"language" => writeback.language.is_some(),
		b"description" => writeback.summary.is_some(),
		b"subject" => !writeback.subjects.is_empty(),
		b"date" => writeback.date.is_some(),
		_ => false,
	}
}

/// Whether a `<meta>` start tag is one of the calibre series tags we replace
fn is_replaced_meta(writeback: &OpfWriteback, tag: &BytesStart) -> bool {
	let Some(name) = tag.attributes().flatten().find_map(|attribute| {
		(attribute.key.local_name().as_ref() == b"name")
			.then(|| String::from_utf8_lossy(&attribute.value).to_string())
	}) else {
		return false;
	};
	(name == "calibre:series" && writeback.series.is_some())
		|| (name == "calibre:series_index" && writeback.series_index.is_some())
}

fn write_simple_element(
	writer: &mut Writer<&mut Vec<u8>>,
	name: &str,
	text: &str,
) -> Result<(), FileError> {
	writer
		.write_event(Event::Start(BytesStart::new(name)))
		.and_then(|_| writer.write_event(Event::Text(BytesText::new(text))))
		.and_then(|_| writer.write_event(Event::End(BytesEnd::new(name))))
		.map_err(|e| FileError::EpubReadError(e.to_string()))
}

fn write_meta_element(
	writer: &mut Writer<&mut Vec<u8>>,
	name: &str,
	content: &str,
) -> Result<(), FileError> {
	let mut tag = BytesStart::new("meta");
	tag.push_attribute(("name", name));
	tag.push_attribute(("content", content));
	writer
		.write_event(Event::Empty(tag))
		.map_err(|e| FileError::EpubReadError(e.to_string()))
}

/// Rewrite the `<metadata>` block of an OPF document: drop the elements we
/// replace, append our values just before `</metadata>`, preserve everything
/// else untouched
fn rewrite_opf(opf: &str, writeback: &OpfWriteback) -> Result<Vec<u8>, FileError> {
	let mut reader = Reader::from_str(opf);
	reader.config_mut().trim_text(false);
	let mut output: Vec<u8> = Vec::with_capacity(opf.len() + 1024);
	let mut writer = Writer::new(&mut output);

	let mut buf = Vec::new();
	let mut inside_metadata = false;
	// Depth of an element currently being skipped (replaced)
	let mut skipping_depth = 0usize;

	loop {
		let event = reader
			.read_event_into(&mut buf)
			.map_err(|e| FileError::EpubReadError(e.to_string()))?;

		match &event {
			Event::Start(tag) => {
				let local = tag.local_name().as_ref().to_vec();
				if skipping_depth > 0 {
					skipping_depth += 1;
				} else if inside_metadata
					&& (is_replaced_element(writeback, &local)
						|| (local == b"meta" && is_replaced_meta(writeback, tag)))
				{
					skipping_depth = 1;
				} else {
					if local == b"metadata" {
						inside_metadata = true;
					}
					writer
						.write_event(event.clone())
						.map_err(|e| FileError::EpubReadError(e.to_string()))?;
				}
			},
			Event::Empty(tag) => {
				let local = tag.local_name().as_ref().to_vec();
				let replaced = inside_metadata
					&& skipping_depth == 0
					&& (is_replaced_element(writeback, &local)
						|| (local == b"meta" && is_replaced_meta(writeback, tag)));
				if skipping_depth == 0 && !replaced {
					writer
						.write_event(event.clone())
						.map_err(|e| FileError::EpubReadError(e.to_string()))?;
				}
			},
			Event::End(tag) => {
				let local = tag.local_name().as_ref().to_vec();
				if skipping_depth > 0 {
					skipping_depth -= 1;
				} else {
					if local == b"metadata" && inside_metadata {
						// Emit our replacement values right before </metadata>
						if let Some(title) = &writeback.title {
							write_simple_element(&mut writer, "dc:title", title)?;
						}
						for creator in &writeback.creators {
							write_simple_element(&mut writer, "dc:creator", creator)?;
						}
						if let Some(publisher) = &writeback.publisher {
							write_simple_element(&mut writer, "dc:publisher", publisher)?;
						}
						if let Some(language) = &writeback.language {
							write_simple_element(&mut writer, "dc:language", language)?;
						}
						if let Some(summary) = &writeback.summary {
							write_simple_element(&mut writer, "dc:description", summary)?;
						}
						for subject in &writeback.subjects {
							write_simple_element(&mut writer, "dc:subject", subject)?;
						}
						if let Some(date) = &writeback.date {
							write_simple_element(&mut writer, "dc:date", date)?;
						}
						if let Some(series) = &writeback.series {
							write_meta_element(&mut writer, "calibre:series", series)?;
						}
						if let Some(series_index) = &writeback.series_index {
							write_meta_element(
								&mut writer,
								"calibre:series_index",
								series_index,
							)?;
						}
						inside_metadata = false;
					}
					writer
						.write_event(event.clone())
						.map_err(|e| FileError::EpubReadError(e.to_string()))?;
				}
			},
			Event::Eof => break,
			_ => {
				if skipping_depth == 0 {
					writer
						.write_event(event.clone())
						.map_err(|e| FileError::EpubReadError(e.to_string()))?;
				}
			},
		}
		buf.clear();
	}

	Ok(output)
}

/// Write the given metadata into the epub at `path`. The archive is rebuilt to
/// a temp file in the same directory and atomically renamed over the original.
/// With `backup` set, the original is first copied to `<path>.bak`
pub fn write_metadata_to_epub(
	path: &str,
	writeback: &OpfWriteback,
	backup: bool,
) -> Result<(), FileError> {
	if writeback.is_empty() {
		tracing::debug!(path, "No metadata values to write, skipping");
		return Ok(());
	}
	if !path.to_lowercase().ends_with(".epub") {
		return Err(FileError::UnsupportedFileType(
			"Metadata writeback only supports epub files".to_string(),
		));
	}

	let file = fs::File::open(path)?;
	let mut archive = ZipArchive::new(file).map_err(FileError::ZipFileError)?;

	let opf_path = locate_opf_path(&mut archive)?;
	let mut opf_contents = String::new();
	archive
		.by_name(&opf_path)
		.map_err(|e| FileError::EpubReadError(e.to_string()))?
		.read_to_string(&mut opf_contents)?;

	let new_opf = rewrite_opf(&opf_contents, writeback)?;

	// Rebuild the archive: mimetype must stay first and uncompressed, the OPF
	// is replaced, every other entry is copied through unchanged
	let parent = Path::new(path)
		.parent()
		.map(Path::to_path_buf)
		.unwrap_or_else(|| PathBuf::from("."));
	let temp_path = parent.join(format!(
		".{}.stump-tmp",
		Path::new(path)
			.file_name()
			.unwrap_or_default()
			.to_string_lossy()
	));

	let result = (|| -> Result<(), FileError> {
		let temp_file = fs::File::create(&temp_path)?;
		let mut zip_writer = ZipWriter::new(temp_file);

		if let Ok(mut mimetype_entry) = archive.by_name("mimetype") {
			let mut mimetype = Vec::new();
			mimetype_entry.read_to_end(&mut mimetype)?;
			drop(mimetype_entry);
			zip_writer
				.start_file(
					"mimetype",
					SimpleFileOptions::default()
						.compression_method(CompressionMethod::Stored),
				)
				.map_err(FileError::ZipFileError)?;
			zip_writer.write_all(&mimetype)?;
		}

		for index in 0..archive.len() {
			let entry = archive
				.by_index_raw(index)
				.map_err(FileError::ZipFileError)?;
			let name = entry.name().to_string();
			if name == "mimetype" || name == opf_path {
				continue;
			}
			zip_writer
				.raw_copy_file(entry)
				.map_err(FileError::ZipFileError)?;
		}

		zip_writer
			.start_file(
				opf_path.as_str(),
				SimpleFileOptions::default()
					.compression_method(CompressionMethod::Deflated),
			)
			.map_err(FileError::ZipFileError)?;
		zip_writer.write_all(&new_opf)?;
		zip_writer.finish().map_err(FileError::ZipFileError)?;
		Ok(())
	})();

	if let Err(error) = result {
		let _ = fs::remove_file(&temp_path);
		return Err(error);
	}

	if backup {
		fs::copy(path, format!("{path}.bak"))?;
	}
	fs::rename(&temp_path, path)?;

	tracing::debug!(path, "Wrote metadata into epub");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::filesystem::media::{
		process::FileProcessor, tests::get_test_epub_path, EpubProcessor,
	};

	fn temp_copy_of_fixture(test_name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join("stump-writeback-tests");
		fs::create_dir_all(&dir).unwrap();
		let dest = dir.join(format!("{test_name}-{}.epub", std::process::id()));
		fs::copy(get_test_epub_path(), &dest).unwrap();
		dest
	}

	#[test]
	fn writes_and_round_trips_metadata() {
		let path = temp_copy_of_fixture("roundtrip");
		let path_str = path.to_string_lossy().to_string();

		let writeback = OpfWriteback {
			title: Some("Новое название".to_string()),
			creators: vec!["Автор Один".to_string(), "Автор Два".to_string()],
			publisher: Some("Издатель X".to_string()),
			subjects: vec!["Фантастика".to_string(), "Приключения".to_string()],
			summary: Some("Updated summary".to_string()),
			date: Some("2024-05-01".to_string()),
			..Default::default()
		};
		write_metadata_to_epub(&path_str, &writeback, false).expect("writeback ok");

		// The file must still be a valid epub Stump can process
		let metadata = EpubProcessor::process_metadata(&path_str)
			.expect("still processable")
			.expect("has metadata");
		assert_eq!(metadata.title.as_deref(), Some("Новое название"));
		let writers = metadata.writers.clone().unwrap_or_default();
		assert!(writers.contains(&"Автор Один".to_string()), "{writers:?}");
		assert_eq!(metadata.publisher.as_deref(), Some("Издатель X"));
		let genres = metadata.genres.clone().unwrap_or_default();
		assert!(genres.contains(&"Фантастика".to_string()), "{genres:?}");

		fs::remove_file(&path).ok();
	}

	#[test]
	fn keeps_untouched_fields_and_reader_compatibility() {
		let path = temp_copy_of_fixture("untouched");
		let path_str = path.to_string_lossy().to_string();

		let before = EpubProcessor::process_metadata(&path_str).unwrap().unwrap();
		// Only change the title — language etc. must survive
		let writeback = OpfWriteback {
			title: Some("Only Title Changed".to_string()),
			..Default::default()
		};
		write_metadata_to_epub(&path_str, &writeback, false).expect("writeback ok");

		let after = EpubProcessor::process_metadata(&path_str).unwrap().unwrap();
		assert_eq!(after.title.as_deref(), Some("Only Title Changed"));
		assert_eq!(after.language, before.language);
		assert_eq!(after.writers, before.writers);

		// The book must still open and serve pages (cover)
		EpubProcessor::get_cover(&path_str).expect("cover still readable");

		fs::remove_file(&path).ok();
	}

	#[test]
	fn backup_flag_keeps_original_copy() {
		let path = temp_copy_of_fixture("backup");
		let path_str = path.to_string_lossy().to_string();
		let original_bytes = fs::read(&path).unwrap();

		let writeback = OpfWriteback {
			title: Some("Backed Up".to_string()),
			..Default::default()
		};
		write_metadata_to_epub(&path_str, &writeback, true).expect("writeback ok");

		let backup_path = format!("{path_str}.bak");
		let backup_bytes = fs::read(&backup_path).expect("backup exists");
		assert_eq!(backup_bytes, original_bytes);

		fs::remove_file(&path).ok();
		fs::remove_file(&backup_path).ok();
	}

	#[test]
	fn empty_writeback_is_a_noop() {
		let path = temp_copy_of_fixture("noop");
		let path_str = path.to_string_lossy().to_string();
		let before = fs::read(&path).unwrap();

		write_metadata_to_epub(&path_str, &OpfWriteback::default(), false)
			.expect("noop ok");
		assert_eq!(fs::read(&path).unwrap(), before);

		fs::remove_file(&path).ok();
	}
}
