use cli::{handle_command, Cli, Parser};
use errors::EntryError;
use stump_core::{
	config::bootstrap_config_dir, config::logging::init_tracing, StumpCore,
};

mod config;
mod errors;
mod http_server;
mod middleware;
mod routers;
mod utils;

// On the (glibc) Linux server build, route Rust allocations through jemalloc so
// its decay settings (see MALLOC_CONF in docker/Dockerfile) return memory to the
// OS after scan peaks. Gated to gnu-linux to match the Cargo.toml dependency; on
// macOS dev builds this is absent and the system allocator is used.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// Cap the Tokio blocking pool well below its 512 default. Cover/thumbnail decodes
// run on spawn_blocking; bounding the pool guards against a runaway number of
// concurrent in-flight image buffers (the scanner's own concurrency is also
// capped, see DEFAULT_MAX_SCANNER_CONCURRENCY).
const MAX_BLOCKING_THREADS: usize = 128;

#[cfg(debug_assertions)]
fn debug_setup() {
	std::env::set_var(
		"STUMP_CLIENT_DIR",
		env!("CARGO_MANIFEST_DIR").to_string() + "/../web/dist",
	);
	std::env::set_var("STUMP_PROFILE", "debug");
	std::env::set_var("STUMP_COLORFUL_LOGS", "true");
}

fn main() -> Result<(), EntryError> {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.max_blocking_threads(MAX_BLOCKING_THREADS)
		.build()
		.expect("failed to build the Tokio runtime")
		.block_on(run())
}

async fn run() -> Result<(), EntryError> {
	#[cfg(debug_assertions)]
	debug_setup();

	let config_dir = bootstrap_config_dir();

	let config = StumpCore::init_config(config_dir)
		.map_err(|e| EntryError::InvalidConfig(e.to_string()))?;

	let cli = Cli::parse();

	if let Some(command) = cli.command {
		Ok(handle_command(command, &cli.config.merge_stump_config(config)).await?)
	} else {
		let resolved_config = cli.config.merge_stump_config(config);
		// Note: init_tracing after loading the environment so the correct verbosity
		// level is used for logging.
		init_tracing(&resolved_config);

		if resolved_config.verbosity >= 3 {
			tracing::trace!(?resolved_config, "App config");
		}

		Ok(http_server::run_http_server(resolved_config).await?)
	}
}
