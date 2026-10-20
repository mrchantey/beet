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
	/// Do not log test cases or file outcomes as they complete.
	pub no_incremental: bool,
	/// Log the name of individual test cases, not files
	pub log_cases: bool,
	/// Log each test name before running it.
	pub log_runs: bool,
	/// Log each skipped test.
	pub log_skipped: bool,
	/// Disable ANSI colored output.
	pub no_color: bool,
	/// Suppress all logger output.
	pub quiet: bool,
	/// Glob pattern filter for test selection. Empty by default (every test
	/// passes); the embedded build can populate it directly since
	/// [`GlobFilter`] is no_std. Patterns are wrapped in wildcards, so `foo`
	/// matches `/foo.rs`, unless `--exact` was passed.
	pub filter: GlobFilter,
	/// Run only ignored tests.
	pub ignored: bool,
	/// Run both ignored and non-ignored tests.
	pub include_ignored: bool,
	/// Timeout per test in milliseconds.
	pub timeout_ms: u64,
	/// Tests in flight at once, libtest's `--test-threads`. Here they are
	/// tasks interleaved on the one runner thread, so the cap is what keeps a
	/// test's timeout measuring its own work rather than the whole suite's.
	pub test_threads: usize,
}

impl Default for TestRunnerConfig {
	fn default() -> Self {
		Self {
			started: Instant::now(),
			watch: false,
			no_incremental: false,
			log_cases: false,
			log_runs: false,
			log_skipped: false,
			no_color: false,
			quiet: false,
			filter: GlobFilter::default(),
			ignored: false,
			include_ignored: false,
			timeout_ms: Self::DEFAULT_TIMEOUT_MS,
			test_threads: Self::default_test_threads(),
		}
	}
}

impl TestRunnerConfig {
	/// Default per-test timeout in milliseconds.
	const DEFAULT_TIMEOUT_MS: u64 = 5_000;

	/// The default for [`test_threads`](Self::test_threads), as libtest's: the
	/// host's parallelism, or eight where it cannot say.
	fn default_test_threads() -> usize {
		const FALLBACK: usize = 8;
		cfg_if! {
			if #[cfg(feature = "std")] {
				std::thread::available_parallelism()
					.map(core::num::NonZero::get)
					.unwrap_or(FALLBACK)
			} else {
				FALLBACK
			}
		}
	}

	/// Creates a config from a [`CliArgs`] instance.
	#[cfg(feature = "std")]
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
		let log_cases = params.contains_key("log-cases");
		let ignored = params.contains_key("ignored");
		let include_ignored = params.contains_key("include-ignored");

		// Parse timeout
		let timeout_ms = params
			.get("timeout-ms")
			.and_then(|val| val.parse::<u64>().ok())
			.unwrap_or(Self::DEFAULT_TIMEOUT_MS);
		// zero would never start a test
		let test_threads = params
			.get("test-threads")
			.and_then(|val| val.parse::<usize>().ok())
			.unwrap_or_else(Self::default_test_threads)
			.max(1);

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
			log_cases,
			log_runs,
			log_skipped,
			no_color,
			quiet,
			ignored,
			include_ignored,
			filter,
			timeout_ms,
			test_threads,
		}
	}

	/// Creates a config by parsing a CLI-style string. The test-only entry
	/// point: a real run reads the process args through [`Self::from_env`].
	#[cfg(all(test, feature = "std"))]
	pub fn from_cli_str(args: &str) -> Self {
		Self::from_cli_args(CliArgs::parse(args))
	}

	/// Creates a config from environment CLI arguments.
	#[cfg(feature = "std")]
	pub fn from_env() -> Self { Self::from_cli_args(CliArgs::parse_env()) }

	/// Returns the instant this was created.
	pub fn started(&self) -> Instant { self.started }

	/// Timeout per test.
	pub fn timeout(&self) -> Duration { Duration::from_millis(self.timeout_ms) }

	/// Returns true if the given test passes the filter.
	pub(crate) fn passes_filter(&self, test: &super::Test) -> bool {
		self.filter.passes(test.name.to_string())
			|| self.filter.passes(test.source_file)
	}
}
