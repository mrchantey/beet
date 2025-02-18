use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::TemplateWatcher;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::ReadFile;


#[derive(Debug, Parser)]
pub struct WatchTemplates {
	/// File or directory to watch
	#[arg(long, default_value = "src")]
	pub src: PathBuf,
	// keep default in sync with StaticFileRouter
	#[arg(long, default_value = "target/rsx-templates.ron")]
	pub dst: PathBuf,
}


impl WatchTemplates {
	pub async fn run(self) -> Result<()> {
		println!("watching templates at {}", self.src.display());

		fn print_dst(dst: &PathBuf) -> Result<()> {
			let str = ReadFile::to_string(dst)?;
			println!("wrote to {}\n{}", dst.display(), str);
			Ok(())
		}
		let dst = self.dst.clone();
		TemplateWatcher::new(
			BuildRsxTemplateMap::new_with_dst(self.src, self.dst),
			|| {
				print_dst(&dst)?;
				Ok(())
			},
			|| {
				print_dst(&dst)?;
				Ok(())
			},
		)?
		.watch()
		.await
	}
}
