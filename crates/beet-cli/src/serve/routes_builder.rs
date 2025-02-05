use super::cargo_cmd::CargoCmd;
use anyhow::Result;
use beet_router::prelude::BuildRoutesMod;
use beet_router::prelude::BuildRsxTemplates;
use beet_router::prelude::HashRsxFile;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use sweet::prelude::WatchEvent;
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
		self.compile_and_run()?;
		let watcher = FsWatcher::default()
			.with_path(&self.build_routes_mod.routes_dir())
			.with_exclude("*.git*")
			.with_exclude("*target*");
		println!("{:#?}", watcher);
		watcher.watch_async(move |ev| self.on_change(ev)).await?;
		Ok(())
	}

	/// find any reason to `cargo build`, if none, just `cargo run`
	fn on_change(&mut self, ev: WatchEvent) -> Result<()> {
		if !ev.has_mutate() {
			return Ok(());
		}
		terminal::clear()?;
		// println!("{:#?}", ev);
		// return Ok(());
		// 1. handle created and changed
		for path in ev
			.events
			.iter()
			.filter_map(|ev| match ev.kind {
				EventKind::Create(CreateKind::File) => Some(ev),
				EventKind::Modify(ModifyKind::Data(_)) => Some(ev),
				_ => None,
			})
			.map(|ev| &ev.paths)
			.flatten()
		{
			if path.extension().map(|x| x == "rs").unwrap_or_default() {
				let new_hash = HashRsxFile::hash_file(&path)?;
				if let Some(curr_hash) = self.file_cache.get(path) {
					if curr_hash == &new_hash {
						continue;
					}
				}
				self.file_cache.insert(path.clone(), new_hash);
				println!(
					"Watcher::Recompile(RustChanged) - {}",
					path.display()
				);
				return self.compile_and_run();
			} else {
				println!("Watcher::Recompile(NewFile) - {}", path.display());
				return self.compile_and_run();
			}
		}
		// 2. handle removed
		for removed in ev
			.events
			.iter()
			.filter_map(|e| {
				if e.kind.is_remove() {
					Some(&e.paths)
				} else {
					None
				}
			})
			.flatten()
		{
			println!("Watcher::Recompile(Removed) - {}", removed.display());
			return self.compile_and_run();
		}
		println!("Watcher::HotReload");
		self.run()
	}

	fn compile_and_run(&mut self) -> Result<()> {
		let start = Instant::now();
		self.build_routes_mod.build_and_write()?;
		self.build_templates.build_and_write()?;
		self.cargo.spawn()?;
		Command::new(self.exe_path()).status()?;
		println!("Recompiled in {:?}", start.elapsed());
		Ok(())
	}

	fn run(&mut self) -> Result<()> {
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
