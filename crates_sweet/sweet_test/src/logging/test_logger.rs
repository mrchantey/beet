use crate::prelude::*;
use colorize::*;
use std::path::Path;
use std::sync::Arc;
use test::TestDescAndFn;
use anyhow::Result;


pub enum CaseLoggerEnum {
	Case(SweetCaseLogger),
	Vanilla(VanillaCaseLogger),
	File(FileLogger),
}

impl CaseLoggerEnum {
	pub fn new(
		config: Arc<TestRunnerConfig>,
		tests: &[&TestDescAndFn],
	) -> Self {
		match config.format {
			OutputFormat::Case => Self::Case(SweetCaseLogger::default()),
			OutputFormat::Vanilla => {
				Self::Vanilla(VanillaCaseLogger::default())
			}
			OutputFormat::File => Self::File(FileLogger::new(tests)),
		}
	}
	// fn as_ref(&self) -> &dyn CaseLogger {
	// 	match self {
	// 		CaseLoggerEnum::CaseSweetly(logger) => logger,
	// 	}
	// }
	fn as_mut(&mut self) -> &mut dyn CaseLogger {
		match self {
			CaseLoggerEnum::Case(logger) => logger,
			CaseLoggerEnum::Vanilla(logger) => logger,
			CaseLoggerEnum::File(logger) => logger,
		}
	}
}

impl CaseLogger for CaseLoggerEnum {
	fn on_result(&mut self, result: &mut TestDescAndResult) -> Result<()> {
		self.as_mut().on_result(result)
	}
	fn end_str(&mut self) -> Option<String> { self.as_mut().end_str() }
}

/// This trait is how you can customize sweet to look how you want
pub trait CaseLogger {
	/// here you can mutate the output of failing results
	fn on_result(&mut self, result: &mut TestDescAndResult) -> Result<()>;
	fn end_str(&mut self) -> Option<String> { None }
}

/// format a file string in a pretty jest style,
/// where the stem is bold and the dir is faint
pub fn pretty_file_path(file: &str) -> String {
	let file = Path::new(file);

	let name = file
		.file_name()
		.unwrap_or_default()
		.to_string_lossy()
		.to_string()
		.bold();
	let dir = file
		.parent()
		.unwrap_or_else(|| Path::new(""))
		.to_string_lossy()
		.to_string()
		.faint();
	let slash = "/".faint();
	format!("{dir}{slash}{name}")
}
