mod apply;
mod fetch;
mod fetch_job;
mod provider_cache;
mod spec;
mod writer;

pub use apply::{apply_media_match, apply_series_match, find_auto_apply_candidate};
pub use fetch::{fetch_media_metadata, fetch_series_metadata};
pub use fetch_job::{
	MetadataFetchJob, MetadataFetchJobOutput, MetadataFetchJobParams, MetadataFetchScope,
};
pub use provider_cache::{ProviderCacheError, ProviderClientCache};
pub use writer::update_embedded_metadata;
