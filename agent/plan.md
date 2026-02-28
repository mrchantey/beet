

## TestRunnerConfig

Lets flatten all our test runner params into a single struct.

Rename TestRunnerArgs to TestRunnerConfig. Remove TestParamQuery.
TestRunnerConfig should be typed, parsing its needed args when created:
```rust
struct TestRunnerArgs{
	pub started: Instant,
	/// Clear the terminal on run and always exit ok for cleaner output when in watch mode
	pub watch: bool,
	/// Do not log test outcomes as they complete
	pub no_incremental: bool,
	/// Log each test name before running it
	pub log_runs: bool,
	/// Log each skipped test
	pub log_skipped: bool,
	/// Disable ANSII colored output
	pub no_color: bool,
	/// Suppress all logger output
	pub quiet: bool,
	/// Glob pattern filter for test selection.
	pub filter: GlobFilter,
	/// By default the glob filter will wrap
	/// all patterns in wildcards, so `*foo*` will match `/foo.rs`.
	/// Specify `--exact` to disable this, ensuring an exact match.
	pub exact: bool,
}
```


- remove TestParamQuery, RunnerParams, . just store the types it needs directly on TestRunnerAr