use super::cargo_cmd::CargoCmd;
use anyhow::Result;
use beet_router::prelude::HashRsxFile;
use std::collections::HashMap;
use std::path::PathBuf;
use sweet::prelude::WatchEvent;
use sweet::prelude::*;


/// Determine if the rust code changed in a file, or if it was just the html
#[derive(Default)]
pub struct RoutesBuilder {
	// we will be swapping out the `run` and `build` methods of this command,
	// depending on the diff
	cargo: CargoCmd,
	watch_dir: PathBuf,
	file_cache: HashMap<PathBuf, u64>,
}

impl RoutesBuilder {
	pub fn new(watch_dir: PathBuf, run_opts: CargoCmd) -> Self {
		Self {
			cargo: run_opts,
			watch_dir,
			file_cache: Default::default(),
		}
	}

	pub fn watch(mut self) -> tokio::task::JoinHandle<Result<()>> {
		tokio::spawn(async move {
			FsWatcher::default()
				.with_path(&self.watch_dir)
				.with_ignore("*target*")
				.watch_async(move |ev| self.on_change(ev))
				.await
		})
	}


	/// find any reason to `cargo build`, if none, just `cargo run`
	fn on_change(&mut self, ev: WatchEvent) -> Result<()> {
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
				return self.cargo.cargo_build();
			} else {
				println!("Watcher::Recompile(NewFile) - {}", path.display());
				return self.cargo.cargo_build();
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
		}
		println!("Watcher::HotReload");
		self.cargo.cargo_run()
	}
}
