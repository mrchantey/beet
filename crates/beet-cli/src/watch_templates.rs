use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::TemplateWatcher;
use clap::Parser;
use std::path::PathBuf;


#[derive(Debug, Parser)]
pub struct WatchTemplates {
	#[arg(long, default_value = "src")]
	pub src: PathBuf,
	// keep default in sync with StaticFileRouter
	#[arg(long, default_value = "target/rsx-templates.ron")]
	pub dst: PathBuf,
}


impl WatchTemplates {
	pub async fn run(self) -> Result<()> {
		TemplateWatcher::new(
			BuildRsxTemplateMap::new(self.src, self.dst),
			|| Ok(()),
			|| Ok(()),
		)?
		.watch()
		.await
	}
}
