use anyhow::Result;
use beet_utils::prelude::GlobFilter;
use clap::Parser;
use clap::ValueEnum;
use glob::Pattern;
use std::str::FromStr;
use test::ShouldPanic;
use test::TestDesc;
use test::TestDescAndFn;
#[allow(unused_imports)]
extern crate test;

/// This is intended to eventually be a superset of the default runner, with options for:
/// - [cargo test cli args](https://doc.rust-lang.org/cargo/commands/cargo-test.html),
/// - [libtest cli args](https://doc.rust-lang.org/rustc/tests/index.html)
///
#[derive(Debug, Default, Clone, Parser)]
pub struct TestRunnerConfig {
	/// A glob pattern to match test names against, by default these are wrapped in stars
	/// but that can be disabled by passing `--exact`.
	#[command(flatten)]
	pub filter: GlobFilter,
	/// Shorthand for --include
	#[arg(trailing_var_arg = true,value_parser = GlobFilter::parse_glob_pattern)]
	pub also_include: Vec<Pattern>,
	#[arg(long)]
	/// Runs only tests that are marked with the [ignore](test::ignore) attribute.
	pub ignored: bool,
	#[arg(long)]
	/// Runs both ignored and non-ignored tests.
	pub include_ignored: bool,
	#[arg(long, default_value_t = true)]
	/// Do not silence stdout
	pub nocapture: bool,
	#[arg(long)]
	/// Excludes tests marked with the [should_panic](test::should_panic) attribute.
	pub exclude_should_panic: bool,
	/// In watch mode we dont want an exit code, it just muddy's the output.
	#[arg(short, long)]
	pub watch: bool,
	/// Save shapshots for tests that pass the filter, instead of matching them.
	#[arg(short, long)]
	pub snapshot: bool,
	#[arg(short, long)]
	pub quiet: bool,
	/// The output format to use: 'file', 'case', 'vanilla'
	#[clap(long, value_enum, default_value_t)]
	pub format: OutputFormat,
	// pub nocapture: bool,
	// #[arg(short, long, action = clap::ArgAction::Count)]
	// verbose: u8,
	/// Number of test threads to run, defaults to max available.
	#[arg(long)]
	pub test_threads: Option<usize>,
	/// Spin up chromedriver for the duration of the tests
	#[arg(long)]
	pub e2e: bool,
	// /// TODO
	// #[arg(long)]
	// report_time: bool,
	// pub logfile: Option<PathBuf>,
}

impl TestRunnerConfig {
	fn parse_inner(mut args: Self) -> Self {
		args.filter.include.extend(
			std::mem::take(&mut args.also_include)
				.into_iter()
				.filter(|p| {
					!p.as_str().starts_with("--")
						&& !p.as_str().starts_with("-")
				}),
		);
		args.filter.wrap_all_with_wildcard();
		args
	}


	/// Same as `clap::parse` but performing inner parsing step.
	pub fn from_env_args() -> Self { Self::parse_inner(Self::parse()) }

	/// Same as `clap::parse_from` but performing inner parsing step.
	pub fn from_raw_args(args: impl Iterator<Item = String>) -> Self {
		Self::parse_inner(Self::parse_from(args))
	}

	/// Checks against ignore, should_panic and filter flags.
	/// If a test should not run, an ignore message is returned
	pub fn should_not_run(&self, test: &TestDescAndFn) -> Option<&'static str> {
		if let Some(ignore_message) =
			self.should_not_run_ignore_flags(&test.desc)
		{
			Some(ignore_message)
		} else if !self.passes_exclude_should_panic(&test.desc) {
			Some("test should panic")
		} else if !self.passes_filters(&test.desc) {
			Some("test does not match filter")
		} else {
			None
		}
	}

	/// Returns true if the test should run
	fn should_not_run_ignore_flags(
		&self,
		desc: &TestDesc,
	) -> Option<&'static str> {
		if self.include_ignored {
			None
		} else if self.ignored && !desc.ignore {
			Some("ignoring tests without #[ignore]")
		} else if !self.include_ignored && desc.ignore {
			if let Some(ignore_message) = desc.ignore_message {
				Some(ignore_message)
			} else {
				Some("test is ignored")
			}
		} else {
			None
		}
	}

	/// Returns true if the test should run
	fn passes_exclude_should_panic(&self, desc: &TestDesc) -> bool {
		if !self.exclude_should_panic {
			return true;
		}
		match desc.should_panic {
			ShouldPanic::No => true,
			ShouldPanic::Yes => false,
			ShouldPanic::YesWithMessage(_) => false,
		}
	}


	/// Checks both the file path and the full test name
	/// If either the file path or the test name matches the include patterns,
	/// and neither matches the exclude patterns, the filter passes.
	/// for matcher `foo` the following will pass:
	/// - path: `/src/foo/bar.rs`
	/// - name: `crate::foo::test::it_works`
	fn passes_filters(&self, desc: &TestDesc) -> bool {
		let file = desc.source_file;
		let name = desc.name.to_string();

		(self.filter.passes_include(&file) || self.filter.passes_include(&name))
			&& self.filter.passes_exclude(&file)
			&& self.filter.passes_exclude(&name)
	}
}




impl std::fmt::Display for TestRunnerConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut messages = Vec::new();
		if self.watch {
			messages.push(format!("watch: true"));
		}
		if self.format != OutputFormat::File {
			messages.push(format!("format: {}", self.format));
		}
		if !self.filter.is_empty() {
			messages.push(format!("matching: {}", self.filter));
		}
		if self.quiet {
			messages.push(format!("quiet: true"));
		}
		if let Some(threads) = self.test_threads {
			messages.push(format!("test threads: {threads}"));
		}

		// if self.verbose > 0 {
		// 	messages.push(format!("verbosity: {}", self.verbose));
		// }
		write!(f, "{}\n", messages.join("\n"))
	}
}

#[derive(Debug, Clone, Default, PartialEq, ValueEnum)]
pub enum OutputFormat {
	/// Output per file
	#[default]
	File,
	Case,
	/// The default test my::test ... ok
	Vanilla,
}

impl FromStr for OutputFormat {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"file" => Ok(Self::File),
			"case" => Ok(Self::Case),
			"vanilla" => Ok(Self::Vanilla),
			_ => Err(format!("unknown output format: {}", s)),
		}
	}
}


impl std::fmt::Display for OutputFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputFormat::File => write!(f, "file"),
			OutputFormat::Case => write!(f, "case"),
			OutputFormat::Vanilla => write!(f, "vanilla"),
		}
	}
}
