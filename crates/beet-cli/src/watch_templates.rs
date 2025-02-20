use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::TemplateWatcher;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::ReadFile;


#[derive(Debug, Parser)]
pub struct WatchTemplates {
	/// do not log the entire file
	#[arg(short, long)]
	pub quiet: bool,
	// keep default in sync with StaticFileRouter
	#[arg(long, default_value = BuildRsxTemplateMap::DEFAULT_TEMPLATES_DST)]
	pub dst: PathBuf,
	/// File or directory to watch
	#[arg(default_value = "src")]
	pub src: PathBuf,
}

impl WatchTemplates {
	pub async fn run(self) -> Result<()> {
		if !self.quiet {
			println!("watching templates at {}", self.src.display());
		}

		let print_dst = || -> Result<()> {
			if self.quiet {
				// println!("
				return Ok(());
			}
			let str = ReadFile::to_string(&self.dst)?;
			println!("wrote to {}\n{}", self.dst.display(), str);
			Ok(())
		};

		TemplateWatcher::new(
			BuildRsxTemplateMap::new_with_dst(self.src, &self.dst),
			|| {
				print_dst()?;
				Ok(())
			},
			|| {
				print_dst()?;
				Ok(())
			},
		)?
		.recompile_and_watch()
		.await
	}
}
