use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap as HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use sweet::prelude::*;


/// Watch the [BuildRsxTemplateMap::src] for changes, and determine if the rust code
/// changed in a file, or if it was just the html template.
/// 
/// The reload step is as follows:
/// 1. rebuild templates
/// 2. call reload
/// 
/// The recompile step is as follows:
/// 1. call recompile
/// 2. rebuild templates
/// 3. call reload
/// 
pub struct TemplateWatcher<Reload, Recompile> {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	build_templates: BuildRsxTemplateMap,
	reload_func: Reload,
	recompile_func: Recompile,
	file_cache: HashMap<PathBuf, u64>,
}

impl<Reload: FnMut() -> Result<()>, Recompile: FnMut() -> Result<()>>
	TemplateWatcher<Reload, Recompile>
{
	pub fn new(
		build_templates: BuildRsxTemplateMap,
		reload_func: Reload,
		recompile_func: Recompile,
	) -> Result<Self> {
		let file_cache = Self::preheat_cache(&build_templates.src)?;
		Ok(Self {
			build_templates,
			file_cache,
			reload_func,
			recompile_func,
		})
	}

	pub async fn watch(mut self) -> Result<()> {
		FsWatcher::default()
			.with_path(&self.build_templates.src)
			.with_exclude("*.git*")
			.with_exclude("*target*")
			.watch_async(move |ev| self.on_change(ev))
			.await
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
		println!("Preheated cache in {:?}", now.elapsed());
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
							// println!(
							// 	"the hash changed\nprev: {}\nnew: {}",
							// 	curr_hash, new_hash
							// );
						}
						self.file_cache.insert(ev.path.clone(), new_hash);
						return self.recompile_then_reload(&ev.display());
					} else {
						return self.recompile_then_reload(&ev.display());
					}
				}
				EventKind::Remove(RemoveKind::File)
				| EventKind::Remove(RemoveKind::Folder) => {
					return self.recompile_then_reload(&ev.display());
				}
				_ => {}
			}
		}

		if let Some(reason) = hotreload_reason {
			return self.reload(&reason);
		}
		Ok(())
	}

	fn recompile_then_reload(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::Recompile: {}", reason);
		let start = Instant::now();
		(self.recompile_func)()?;
		self.build_templates.build_and_write()?;
		(self.reload_func)()?;
		println!("Watcher::Recompile Duration: {:?}", start.elapsed());
		Ok(())
	}

	fn reload(&mut self, reason: &str) -> Result<()> {
		println!("Watcher::Reload: {}", reason);
		let start = Instant::now();
		// first rebuild templates
		self.build_templates.build_and_write()?;
		(self.reload_func)()?;
		println!("Watcher::Reload Duration: {:?}", start.elapsed());
		Ok(())
	}
}
