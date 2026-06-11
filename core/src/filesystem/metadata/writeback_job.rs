//! A background job which writes the database metadata of every epub in a
//! library back into the files themselves (see
//! [`crate::filesystem::media::epub_writeback`])

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use models::entity::{media, media_metadata, series};
use sea_orm::{prelude::*, ActiveValue::Set, IntoActiveModel, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::{
	database::chunk_vec_into,
	filesystem::media::{
		epub_writeback::{write_metadata_to_epub, OpfWriteback},
		EpubProcessor, FileProcessor, FileProcessorOptions,
	},
	job::{
		error::JobError, JobContext, JobLifecycle, JobOutputExt, JobProgress,
		JobTaskOutput, WorkingState,
	},
};

type Id = String;

/// Write one book's stored metadata into its epub file and refresh the file
/// state (mtime, size, hashes) kept in the database so the scanner does not
/// treat the rewrite as an external modification. Returns false when there was
/// nothing to write
pub async fn write_back_book(
	conn: &DatabaseConnection,
	book: &media::Model,
	backup: bool,
) -> Result<bool, JobError> {
	let Some(metadata) = media_metadata::Entity::find()
		.filter(media_metadata::Column::MediaId.eq(book.id.clone()))
		.one(conn)
		.await?
	else {
		return Ok(false);
	};

	let writeback = OpfWriteback::from(&metadata);
	let path = book.path.clone();
	let backup_flag = backup;
	let did_write = tokio::task::spawn_blocking({
		let writeback = writeback.clone();
		let path = path.clone();
		move || -> Result<bool, JobError> {
			if writeback.is_empty() {
				return Ok(false);
			}
			write_metadata_to_epub(&path, &writeback, backup_flag)
				.map_err(|e| JobError::TaskFailed(e.to_string()))?;
			Ok(true)
		}
	})
	.await
	.map_err(|e| JobError::TaskFailed(e.to_string()))??;

	if !did_write {
		return Ok(false);
	}

	// Refresh the stored file state so the next scan doesn't re-import the book
	let fs_metadata = tokio::fs::metadata(&path)
		.await
		.map_err(|e| JobError::TaskFailed(e.to_string()))?;
	let modified_at = fs_metadata
		.modified()
		.ok()
		.map(|time| DateTime::<Utc>::from(time).into());

	let hashes = EpubProcessor::generate_hashes(
		&path,
		FileProcessorOptions {
			generate_file_hashes: book.hash.is_some(),
			generate_koreader_hashes: book.koreader_hash.is_some(),
			..Default::default()
		},
	)
	.map_err(|e| JobError::TaskFailed(e.to_string()))?;

	let mut active = book.clone().into_active_model();
	active.size = Set(fs_metadata.len() as i64);
	if modified_at.is_some() {
		active.modified_at = Set(modified_at);
	}
	if book.hash.is_some() {
		active.hash = Set(hashes.hash);
	}
	if book.koreader_hash.is_some() {
		active.koreader_hash = Set(hashes.koreader_hash);
	}
	active.update(conn).await?;

	Ok(true)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetadataWritebackJobParams {
	pub library_id: Id,
	pub backup: bool,
}

#[derive(Serialize, Deserialize)]
pub enum MetadataWritebackTask {
	Batch(Vec<Id>),
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, SimpleObject)]
#[serde(default, rename_all = "camelCase")]
pub struct MetadataWritebackOutput {
	/// Books whose files were updated
	pub written: u64,
	/// Books skipped (not epub, no metadata, or nothing to write)
	pub skipped: u64,
	/// Books that failed to write (see logs)
	pub failed: u64,
}

impl JobOutputExt for MetadataWritebackOutput {
	fn update(&mut self, updated: Self) {
		self.written += updated.written;
		self.skipped += updated.skipped;
		self.failed += updated.failed;
	}
}

/// The job which sweeps a library and writes stored metadata into every epub
#[derive(Clone)]
pub struct MetadataWritebackJob {
	pub params: MetadataWritebackJobParams,
}

#[async_trait::async_trait]
impl JobLifecycle for MetadataWritebackJob {
	const NAME: &'static str = "metadata_writeback";

	type Output = MetadataWritebackOutput;
	type Task = MetadataWritebackTask;

	fn description(&self) -> Option<String> {
		Some(format!(
			"Write metadata into epub files, library {}, backup: {}",
			self.params.library_id, self.params.backup
		))
	}

	async fn init(
		&mut self,
		ctx: &JobContext,
	) -> Result<WorkingState<Self::Output, Self::Task>, JobError> {
		let media_ids: Vec<Id> = media::Entity::find()
			.select_only()
			.column(media::Column::Id)
			.inner_join(series::Entity)
			.filter(series::Column::LibraryId.eq(self.params.library_id.clone()))
			// Case-insensitive: extension is stored verbatim, so a file named
			// "Book.EPUB" has extension "EPUB" (the single-book mutation lowercases too)
			.filter(Expr::cust("lower(\"media\".\"extension\") = 'epub'"))
			.into_tuple()
			.all(ctx.conn())
			.await
			.map_err(|e| JobError::InitFailed(e.to_string()))?;

		ctx.report_progress(JobProgress::msg(&format!(
			"Writing metadata into {} epub files",
			media_ids.len()
		)));

		let tasks = chunk_vec_into(media_ids, MetadataWritebackTask::Batch);

		Ok(WorkingState {
			output: Some(Self::Output::default()),
			tasks: tasks.into(),
			logs: vec![],
		})
	}

	async fn execute_task(
		&self,
		ctx: &JobContext,
		task: Self::Task,
	) -> Result<JobTaskOutput<Self>, JobError> {
		let mut output = Self::Output::default();
		let mut logs = vec![];

		let MetadataWritebackTask::Batch(media_ids) = task;
		let total = media_ids.len() as i32;

		let books = media::Entity::find()
			.filter(media::Column::Id.is_in(media_ids))
			.all(ctx.conn())
			.await
			.map_err(|e| JobError::TaskFailed(e.to_string()))?;

		for (position, book) in books.iter().enumerate() {
			ctx.report_progress(JobProgress::subtask_position_msg(
				"Writing metadata into files",
				position as i32 + 1,
				total,
			));
			match write_back_book(ctx.conn(), book, self.params.backup).await {
				Ok(true) => output.written += 1,
				Ok(false) => output.skipped += 1,
				Err(error) => {
					output.failed += 1;
					logs.push(
						crate::job::JobExecuteLog::error(format!(
							"Failed to write metadata: {error}"
						))
						.with_ctx(book.path.clone()),
					);
				},
			}
		}

		Ok(JobTaskOutput {
			output,
			logs,
			subtasks: vec![],
		})
	}
}
