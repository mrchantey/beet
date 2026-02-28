//! Test runner configuration parsed from CLI input.
//!
//! [`TestRunnerConfig`] is the single source of truth for all test runner
//! parameters, parsed upfront from CLI arguments.

use crate::prelude::*;

/// CLI arguments parsed into a typed configuration for the test runner.
///
/// This is a [`Component`] spawned alongside test bundles. All parameters
/// are parsed eagerly at construction time rather than lazily extracted.
#[derive(Debug, Clone, Component)]
pub struct TestRunnerConfig {
	/// The instant this was created, for timing.
	started: Instant,
	/// Clear the terminal on run and always exit ok for cleaner output when in watch mode.
	pub watch: bool,
	/// Do not log test outcomes as they complete.
	pub no_incremental: bool,
	/// Log each test name before running it.
	pub log_runs: bool,
	/// Log each skipped test.
	pub log_skipped: bool,
	/// Disable ANSI colored output.
	pub no_color: bool,
	/// Suppress all logger output.
	pub quiet: bool,
	/// Glob pattern filter for test selection.
	pub filter: GlobFilter,
	/// By default the glob filter wraps all patterns in wildcards,
	/// so `*foo*` will match `/foo.rs`. Specify `--exact` to disable this.
	pub exact: bool,
	/// Timeout per test in milliseconds.
	pub timeout_ms: u64,
}

impl TestRunnerConfig {
	/// Default per-test timeout in milliseconds.
	const DEFAULT_TIMEOUT_MS: u64 = 5_000;

	/// Creates a config from a [`CliArgs`] instance.
	pub fn from_cli_args(args: CliArgs) -> Self {
		let params = args.params;

		// while Multimap does have deserialization capabilities through bevy reflect,
		// our merging of positional args to include filters requires a custom impl

		// Parse boolean flags
		let watch = params.contains_key("watch");
		let no_incremental = params.contains_key("no-incremental");
		let log_runs = params.contains_key("log-runs");
		let log_skipped = params.contains_key("log-skipped");
		let no_color = params.contains_key("no-color");
		let quiet = params.contains_key("quiet");
		let exact = params.contains_key("exact");

		// Parse timeout
		let timeout_ms = params
			.get("timeout-ms")
			.and_then(|val| val.parse::<u64>().ok())
			.unwrap_or(Self::DEFAULT_TIMEOUT_MS);

		// Build the glob filter from named params and positional args
		let mut filter = GlobFilter::default();
		if let Some(includes) = params.get_vec("include") {
			filter = filter.extend_include(includes);
		}
		if let Some(excludes) = params.get_vec("exclude") {
			filter = filter.extend_exclude(excludes);
		}
		// Extend include by positional args
		filter = filter.extend_include(&args.path);
		// Wrap patterns in wildcards unless exact mode
		if !exact {
			filter.wrap_all_with_wildcard();
		}

		Self {
			started: Instant::now(),
			watch,
			no_incremental,
			log_runs,
			log_skipped,
			no_color,
			quiet,
			exact,
			filter,
			timeout_ms,
		}
	}

	/// Creates a config by parsing a CLI-style string.
	pub fn from_cli_str(args: &str) -> Self {
		Self::from_cli_args(CliArgs::parse(args))
	}

	/// Creates a config from environment CLI arguments.
	pub fn from_env() -> Self { Self::from_cli_args(CliArgs::parse_env()) }

	/// Returns the instant this was created.
	pub fn started(&self) -> Instant { self.started }

	/// Timeout per test.
	pub fn timeout(&self) -> Duration { Duration::from_millis(self.timeout_ms) }

	/// Returns true if the given test passes the filter.
	pub fn passes_filter(&self, test: &super::Test) -> bool {
		self.filter.passes(test.name.to_string())
			|| self.filter.passes(test.source_file)
	}
}
