use super::cargo_cmd::CargoCmd;
use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::CollectRoutes;
use beet_router::prelude::HashRsxFile;
use rapidhash::RapidHashMap as HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use sweet::prelude::*;


/// Watch the [CollectRoutes::src_dir] for changes, and determine if the rust code
/// changed in a file, or if it was just the html template
#[derive(Default)]
pub struct RoutesBuilder {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	cargo: CargoCmd,
	collect_routes: CollectRoutes,
	build_templates: BuildRsxTemplateMap,
	file_cache: HashMap<PathBuf, u64>,
}

impl RoutesBuilder {
	pub fn new(
		collect_routes: CollectRoutes,
		mut cargo: CargoCmd,
	) -> Result<Self> {
		cargo.cargo_cmd = "build".to_string();
		let mut build_templates = BuildRsxTemplateMap {
			pretty: true,
			..Default::default()
		};
		build_templates.src = collect_routes.src_dir().clone();
		let file_cache = Self::preheat_cache(collect_routes.src_dir())?;
		Ok(Self {
			cargo,
			build_templates,
			collect_routes,
			file_cache,
		})
	}

	pub async fn watch(mut self) -> Result<()> {
		let watcher = FsWatcher::default()
			.with_path(&self.collect_routes.src_dir())
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

	/// Create a file cache with every file in the src directory
	pub fn preheat_cache(src: &Path) -> Result<HashMap<PathBuf, u64>> {
		let mut cache = HashMap::default();
		let files = ReadDir::files_recursive(src)?;
		let now = Instant::now();
		for file in files {
			let path = file.canonicalize()?;
			let hash = HashRsxFile::file_to_hash(&path)?;
			cache.insert(path, hash);
		}
		println!("Preheated cache in {:?}\n{:#?}", now.elapsed(), cache);
		Ok(cache)
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
						let new_hash = HashRsxFile::file_to_hash(&ev.path)?;
						if let Some(curr_hash) = self.file_cache.get(&ev.path) {
							if curr_hash == &new_hash {
								hotreload_reason = Some(ev.display());
								continue;
							}
							println!(
								"the hash changed\nprev: {}\nnew: {}",
								curr_hash, new_hash
							);
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
		// self.collect_routes.build_and_write()?;
		self.cargo.spawn()?;
		self.build_templates.build_and_write()?;
		Command::new(self.exe_path()).status()?;
		println!("Recompiled in {:?}", start.elapsed());
		Ok(())
	}

	fn run(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::HotReload: {}", reason);
		let start = Instant::now();
		self.build_templates.build_and_write()?;
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
