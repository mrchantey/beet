use super::cargo_cmd::CargoCmd;
use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::CollectRoutes;
use beet_router::prelude::TemplateWatcher;
use std::process::Command;


/// Convenice wrapper for a rebuild
#[derive(Default)]
pub struct FileRoutesWatcher {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	cargo: CargoCmd,
	collect_routes: CollectRoutes,
}

impl FileRoutesWatcher {
	pub fn new(
		collect_routes: CollectRoutes,
		mut cargo: CargoCmd,
	) -> Result<Self> {
		cargo.cargo_cmd = "build".to_string();
		Ok(Self {
			cargo,
			collect_routes,
		})
	}

	pub async fn watch(self) -> Result<()> {
		let build_templates = BuildRsxTemplateMap::new_with_dst(
			self.collect_routes.src_dir(),
			"target/rsx-templates.ron",
		);
		let exe_path = self.exe_path();
		let reload = move || -> Result<()> {
			Command::new(&exe_path).status()?;
			Ok(())
		};

		let exe_path = self.exe_path();
		let recompile = move || -> Result<()> {
			// TODO only recollect routes if routes change?
			self.collect_routes.build_and_write()?;
			self.cargo.spawn()?;
			Command::new(&exe_path).status()?;
			Ok(())
		};
		// always compile on first run
		recompile()?;

		TemplateWatcher::new(build_templates, reload, recompile)?
			.watch()
			.await
	}

	fn exe_path(&self) -> String {
		let target_dir = std::env::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".to_string());
		format! {"{target_dir}/debug/beet_site"}
	}
}
