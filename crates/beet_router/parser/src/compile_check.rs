use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;


pub struct CompileCheck;


impl CompileCheck {
	pub fn string(code: &str) -> Result<()> {
		// Create temporary file with unique name
		let temp_dir = std::env::temp_dir();
		let file_name = format!("compilecheck_{}.rs", Uuid::new_v4());
		let file_path = temp_dir.join(&file_name);

		// Write code to temp file
		fs::write(&file_path, code)?;

		// Attempt compilation
		let output = Self::file(&file_path);
		// Clean up temp files
		let _ = fs::remove_file(&file_path);
		let _ = fs::remove_file(
			temp_dir.join(Path::new(&file_name).file_stem().unwrap()),
		);
		output
	}

	pub fn file(path: &Path) -> Result<()> {
		let output = Command::new("rustc").arg(&path).output()?;
		if output.status.success() {
			Ok(())
		} else {
			anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr))
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "need to suppress output files"]
	fn works() {
		expect(CompileCheck::string("fn main(){}")).to_be_ok();
		expect(CompileCheck::string("dsajk923")).to_be_err();
	}
}
