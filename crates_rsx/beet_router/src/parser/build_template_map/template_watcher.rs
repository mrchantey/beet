use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap as HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use sweet::fs::exports::notify::EventKind;
use sweet::fs::exports::notify::event::CreateKind;
use sweet::fs::exports::notify::event::ModifyKind;
use sweet::fs::exports::notify::event::RemoveKind;
use sweet::prelude::*;


/// Watch the [BuildTemplateMap::templates_root_dir] for changes, and determine if the rust code
/// changed in a file, or if it was just the html template.
///
/// The reload step is as follows:
/// 1. call reload
///
/// The recompile step is as follows:
/// 1. call recompile
/// 2. call reload
///
pub struct TemplateWatcher<Reload, Recompile> {
	templates_root_dir: PathBuf,
	reload_func: Reload,
	recompile_func: Recompile,
	/// A hash of the *code parts* of each file being watched.
	/// Used to determine if recompilation is required.
	file_cache: HashMap<PathBuf, u64>,
}

impl<Reload: FnMut() -> Result<()>, Recompile: FnMut() -> Result<()>>
	TemplateWatcher<Reload, Recompile>
{
	pub fn new(
		templates_root_dir: impl AsRef<Path>,
		reload_func: Reload,
		recompile_func: Recompile,
	) -> Result<Self> {
		let file_cache = preheat_cache(&templates_root_dir)?;
		Ok(Self {
			templates_root_dir: templates_root_dir.as_ref().to_path_buf(),
			file_cache,
			reload_func,
			recompile_func,
		})
	}
	pub async fn run_once_and_watch(mut self) -> Result<()> {
		self.recompile_then_reload("Initial build")?;
		self.watch().await?;
		Ok(())
	}

	pub async fn watch(mut self) -> Result<()> {
		FsWatcher {
			cwd: self.templates_root_dir.clone(),
			filter: GlobFilter::default()
				.with_exclude("*.git*")
				.with_exclude("*codegen*") // temp until we get fine grained codegen control
				.with_exclude("*target*"),
			// avoid short burst refreshing
			debounce: Duration::from_millis(100),
			..Default::default()
		}
		.watch_async(move |ev| {
			// on_change errors are not fatal, just print the error
			if let Err(err) = self.on_change(ev) {
				eprintln!("{}", err);
			}
			Ok(())
		})
		.await
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
					let new_hash =
						HashFile::default().file_to_hash(&ev.path)?;
					if let Some(curr_hash) = self.file_cache.get(&ev.path) {
						if curr_hash == &new_hash {
							// no code changed, just reload
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

	/// Call the following:
	/// - [`Self::recompile_func`]
	/// - [`Self::build_templates::build_and_write`]
	/// - [`Self::reload_func`]
	pub fn recompile_then_reload(&mut self, reason: &str) -> Result<()> {
		// terminal::clear()?;
		println!("Watcher::Recompile: {}", reason);
		let start = Instant::now();
		(self.recompile_func)()?;
		(self.reload_func)()?;
		println!("Watcher::Recompile Duration: {:?}", start.elapsed());
		Ok(())
	}

	fn reload(&mut self, reason: &str) -> Result<()> {
		println!("Watcher::Reload: {}", reason);
		let start = Instant::now();
		(self.reload_func)()?;
		println!("Watcher::Reload Duration: {:?}", start.elapsed());
		Ok(())
	}
}


/// Create a file cache with every file in the src directory
fn preheat_cache(src: impl AsRef<Path>) -> Result<HashMap<PathBuf, u64>> {
	// let now = Instant::now();
	// TODO rayon par_iter
	let cache: HashMap<PathBuf, u64> = ReadDir::files_recursive(src)?
		.into_iter()
		.map(|file| {
			let path = file.canonicalize()?;
			let hash = HashFile::default().file_to_hash(&path)?;
			Ok((path, hash))
		})
		.collect::<Result<_>>()?;

	// println!("Preheated cache in {:?}", now.elapsed());
	Ok(cache)
}
#[cfg(test)]
mod test {
	use sweet::prelude::*;

	use super::preheat_cache;

	#[test]
	fn test_preheat_cache() {
		let src = WorkspacePathBuf::new("crates_rsx/beet_router/src/test_site")
			.into_abs()
			.unwrap();
		let cache = preheat_cache(&src).unwrap();

		let num_files = ReadDir::files_recursive(&src).unwrap().len();

		// flaky, depends on number of files in the test_site directory
		expect(cache.len()).to_be(num_files);
		// println!("{:#?}", cache);
	}
}
