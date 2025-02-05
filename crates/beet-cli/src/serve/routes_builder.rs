use super::cargo_cmd::CargoCmd;
use anyhow::Result;
use beet_router::prelude::BuildRoutesMod;
use beet_router::prelude::BuildRsxTemplates;
use beet_router::prelude::HashRsxFile;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use sweet::prelude::*;


/// Determine if the rust code changed in a file, or if it was just the html
#[derive(Default)]
pub struct RoutesBuilder {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	cargo: CargoCmd,
	build_routes_mod: BuildRoutesMod,
	build_templates: BuildRsxTemplates,
	file_cache: HashMap<PathBuf, u64>,
}

impl RoutesBuilder {
	pub fn new(build_routes_mod: BuildRoutesMod, mut cargo: CargoCmd) -> Self {
		cargo.cargo_cmd = "build".to_string();
		let mut build_templates = BuildRsxTemplates::default();
		build_templates.src = build_routes_mod.routes_dir().clone();
		Self {
			cargo,
			build_templates,
			build_routes_mod,
			file_cache: Default::default(),
		}
	}

	pub async fn watch(mut self) -> Result<()> {
		let watcher = FsWatcher::default()
			.with_path(&self.build_routes_mod.routes_dir())
			.with_exclude("*.git*")
			.with_exclude("*target*");
		println!("{:#?}", watcher);

		self.compile_and_run("Init")?;

		watcher
			.watch_async(move |ev| {
				self.on_change(ev).ok_or(|e| {
					eprintln!("{:#?}", e);
				});
				Ok(())
			})
			.await?;
		Ok(())
	}

	/// find any reason to `cargo build`, if none, just `cargo run`
	fn on_change(&mut self, watch_event: WatchEventVec) -> Result<()> {
		if !watch_event.has_mutate() {
			return Ok(());
		}

		let mut hotreload_reason = None;

		for ev in watch_event.events.into_iter() {
			match ev.kind {
				EventKind::Create(CreateKind::File)
				| EventKind::Modify(ModifyKind::Data(_))
				| EventKind::Modify(ModifyKind::Name(_)) => {
					if ev
						.path
						.extension()
						.map(|x| x == "rs")
						.unwrap_or_default()
					{
						let new_hash = HashRsxFile::hash_file(&ev.path)?;
						if let Some(curr_hash) = self.file_cache.get(&ev.path) {
							if curr_hash == &new_hash {
								// match!
								hotreload_reason = Some(ev.display());
								continue;
							}
						}
						self.file_cache.insert(ev.path.clone(), new_hash);
						return self.compile_and_run(&ev.display());
					} else {
						return self.compile_and_run(&ev.display());
					}
				}
				EventKind::Remove(RemoveKind::File)
				| EventKind::Remove(RemoveKind::Folder) => {
					return self.compile_and_run(&ev.display());
				}
				_ => {}
			}
		}

		if let Some(reason) = hotreload_reason {
			return self.run(&reason);
		}
		Ok(())
	}

	fn compile_and_run(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::Recompile: {}", reason);
		let start = Instant::now();
		// 🤪 disable build routes for now
		// self.build_routes_mod.build_and_write()?;
		self.build_templates.build_and_write()?;
		self.cargo.spawn()?;
		Command::new(self.exe_path()).status()?;
		println!("Recompiled in {:?}", start.elapsed());
		Ok(())
	}

	fn run(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::HotReload: {}", reason);
		let start = Instant::now();
		Command::new(self.exe_path()).status()?;
		println!("Ran in {:?}", start.elapsed());
		Ok(())
	}

	fn exe_path(&self) -> String {
		let target_dir = std::env::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".to_string());
		format! {"{target_dir}/debug/beet_site"}
	}

	// fn recompile(&self,
}
