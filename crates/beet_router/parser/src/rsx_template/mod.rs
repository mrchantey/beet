pub use crate::prelude::*;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::ReadDir;
use sweet::prelude::ReadFile;
// use syn::File;


#[derive(Debug, Parser)]
pub struct BuildRsxTemplates {
	#[arg(long, default_value = "src")]
	pub src: PathBuf,
	#[arg(long, default_value = "target/rsx-templates.bin")]
	pub dst: PathBuf,
}



impl Default for BuildRsxTemplates {
	fn default() -> Self { clap::Parser::parse_from(&[""]) }
}

impl BuildRsxTemplates {
	pub fn run(&self) -> Result<()> {
		ReadDir::files_recursive(&self.src)?
			.into_iter()
			.map(|path| self.file_to_partial(path))
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}


	fn file_to_partial(&self, path: PathBuf) -> Result<()> {
		let file = ReadFile::to_string(&path)?;
		let _file = syn::parse_file(&file)?;
		println!("{:?}", path);
		// Ok((path, file))

		// let file = syn::parse_file(&file.to_token_stream().to_string())?;
		Ok(())
	}
}
