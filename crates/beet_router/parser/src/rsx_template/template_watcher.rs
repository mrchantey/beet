use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap as HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use sweet::prelude::*;


/// Watch the [CollectRoutes::src_dir] for changes, and determine if the rust code
/// changed in a file, or if it was just the html template
pub struct TemplateWatcher<Reload, Recompile> {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	build_templates: BuildRsxTemplateMap,
	reload_func: Reload,
	recompile_func: Recompile,
	file_cache: HashMap<PathBuf, u64>,
}

impl<
		Reload: FnMut() -> Result<()>,
		Recompile: FnMut() -> Result<()>,
	> TemplateWatcher<Reload, Recompile>
{
	pub fn new(
		build_templates: BuildRsxTemplateMap,
		reload: Reload,
		recompile: Recompile,
	) -> Result<Self> {
		let file_cache = Self::preheat_cache(&build_templates.src)?;
		Ok(Self {
			build_templates,
			file_cache,
			reload_func: reload,
			recompile_func: recompile,
		})
	}

	pub async fn watch(mut self) -> Result<()> {
		let watcher = FsWatcher::default()
			.with_path(&self.build_templates.src)
			.with_exclude("*.git*")
			.with_exclude("*target*");
		println!("{:#?}", watcher);

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

	/// OnChange will iterate over the watch events,
	/// - if any break the file hash cache, [`Self::recompile`] will be called
	/// - otherwise if it was a file change, [`Self::reload`] will be called
	fn on_change(&mut self, watch_event: WatchEventVec) -> Result<()> {
		// if no file was mutated just exit
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
						return self.recompile(&ev.display());
					} else {
						return self.recompile(&ev.display());
					}
				}
				EventKind::Remove(RemoveKind::File)
				| EventKind::Remove(RemoveKind::Folder) => {
					return self.recompile(&ev.display());
				}
				_ => {}
			}
		}

		if let Some(reason) = hotreload_reason {
			return self.reload(&reason);
		}
		Ok(())
	}

	fn recompile(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::Recompile: {}", reason);
		let start = Instant::now();
		(self.recompile_func)()?;
		println!("Recompiled in {:?}", start.elapsed());
		Ok(())
	}

	fn reload(&mut self, reason: &str) -> Result<()> {
		println!("Watcher::HotReload: {}", reason);
		let start = Instant::now();
		// first rebuild templates
		self.build_templates.build_and_write()?;
		(self.reload_func)()?;
		println!("Ran in {:?}", start.elapsed());
		Ok(())
	}
}
