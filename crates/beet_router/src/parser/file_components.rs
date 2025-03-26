use super::FileGroup;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::WorkspacePathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildFileComponents {
	pub files: FileGroup,
	/// The output codegen file location.
	pub output: WorkspacePathBuf,
}

impl BuildFileComponents {
	pub fn new(files: impl Into<FileGroup>, output: WorkspacePathBuf) -> Self {
		Self {
			files: files.into(),
			output,
		}
	}
}



impl BuildStep for BuildFileComponents {
	fn run(&self) -> Result<()> {
		// self.build_and_write()?;
		Ok(())
	}
}
