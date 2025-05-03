use colorize::*;
use std::path::Path;
use std::path::PathBuf;
// use std::default;

#[derive(Debug, Default)]
pub struct SuiteResult {
	pub file: String,
	pub num_tests: usize,
	pub num_ignored: usize,
	pub failed: Vec<String>,
}


impl SuiteResult {
	pub fn new(file: String, tests: usize, skipped: usize) -> Self {
		SuiteResult {
			file,
			num_tests: tests,
			num_ignored: skipped,
			failed: Vec::new(),
		}
	}
	pub fn with_failed(mut self, failed: Vec<String>) -> Self {
		self.failed = failed;
		self
	}

	pub fn in_progress_str(&self) -> String {
		let mut value = " RUNS ".black().bold().yellowb();
		value += " ";
		value += self.pretty_path().as_str();
		value
	}


	pub fn end_str(&self) -> String {
		let mut val = if self.failed.len() == 0 {
			" PASS ".black().bold().greenb()
		} else {
			" FAIL ".black().bold().redb()
		};
		val += " ";
		val += self.pretty_path().as_str();

		val += &self
			.failed
			.iter()
			.fold(String::new(), |val, err| val + err.to_string().as_str());

		val
	}

	fn pretty_path(&self) -> String {
		let file = PathBuf::from(&self.file);

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
}




// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use sweet::prelude::*;

// 	#[test]
// 	fn works() -> Result<()> {
// 		let file = std::path::Path::new(file!()).to_forward_slash();
// 		let result = SuiteResult::new(file, 0, 0);
// 		let end = result.end_str();
// 		expect(end.as_str()).to_be("\u{1b}[42;1;30m PASS \u{1b}[0;39;49m \u{1b}[2mtest/common\u{1b}[0;39;49m\u{1b}[2m/\u{1b}[0;39;49m\u{1b}[1msuite_result.rs\u{1b}[0;39;49m")?;

// 		Ok(())
// 	}

// }
