use anyhow::Result;
use clap::Parser;
use clap::ValueEnum;
use glob::Pattern;
use glob::PatternError;
use std::path::PathBuf;
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
	#[arg(value_parser= parse_glob)]
	pub filters: Vec<Pattern>,
	/// This forces filters to match the full path of the test exactly.
	#[arg(long)]
	pub exact: bool,
	#[arg(long)]
	/// Runs only tests that are marked with the [ignore](test::ignore) attribute.
	pub ignored: bool,
	#[arg(long)]
	/// Runs both ignored and non-ignored tests.
	pub include_ignored: bool,
	#[arg(long)]
	// /// Do not silence stdout
	// not yet supported
	// pub nocapture: bool,
	#[arg(long)]
	/// Excludes tests marked with the [should_panic](test::should_panic) attribute.
	pub exclude_should_panic: bool,
	/// In watch mode we dont want an exit code, it just muddy's the output.
	#[arg(short, long)]
	pub watch: bool,
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
fn parse_glob(s: &str) -> Result<Pattern, PatternError> {
	Pattern::new(&format!("*{s}*"))
	// Ok(Pattern::new(s)?)
}

impl TestRunnerConfig {
	fn parse_inner(mut args: Self) -> Self {
		if args.exact {
			args.filters = args
				.filters
				.into_iter()
				// remove the leading and trailing stars
				.map(|m| {
					let mut s = m.as_str().to_string();
					s.remove(0);
					s.pop();
					Pattern::new(&s)
				})
				.collect::<Result<Vec<_>, _>>()
				.unwrap();
		}
		args
	}


	/// Same as `clap::parse` but performing inner parsing step.
	pub fn from_env_args() -> Self { Self::parse_inner(Self::parse()) }

	/// Same as `clap::parse_from` but performing inner parsing step.
	pub fn from_raw_args(args: impl Iterator<Item = String>) -> Self {
		Self::parse_inner(Self::parse_from(args))
	}

	pub fn suite_passes_filter(&self, path: &PathBuf) -> bool {
		let matchable_path = path.to_string_lossy();
		self.filters.len() == 0
			|| self.filters.iter().any(|a| a.matches(&matchable_path))
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
	///
	/// for matcher `foo` the following will pass:
	/// - path: `/src/foo/bar.rs`
	/// - name: `crate::foo::test::it_works`
	fn passes_filters(&self, desc: &TestDesc) -> bool {
		if self.filters.len() == 0 {
			return true;
		}
		let path = desc.source_file;
		let name = desc.name.to_string();
		self.filters.iter().any(|a| a.matches(&path))
			|| self.filters.iter().any(|a| a.matches(&name))
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
		if self.filters.len() > 0 {
			let matches = self
				.filters
				.iter()
				.map(|m| m.to_string())
				.collect::<Vec<_>>()
				.join(" ");
			messages.push(format!("matching: {matches}"));
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
