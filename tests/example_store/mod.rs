//! The in-memory store the acceptance suites boot an on-disk example from,
//! shared by the terminal and browser hosts.
//!
//! An example that forks (`<SceneBlob>`, `<DocumentBlob>`) writes beside its
//! sources, so a suite over its on-disk directory would read the fork a real
//! session left behind and rewrite it. Copying the entry and the documents it
//! names off disk into memory keeps every case a first boot over the authored
//! files alone, with the fork and the artifact landing in memory.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test
//! target, so each host `#[path]`-includes this module.
use beet::prelude::*;

/// The files at `paths` within the workspace-relative `dir`, copied off disk
/// into an in-memory store, a symlinked tree (`examples/ui/assets`) included.
pub async fn seeded_store(dir: &str, paths: &[&str]) -> BlobStore {
	let disk =
		BlobStore::new(FsStore::new(AbsPath::new_workspace_rel(dir).unwrap()));
	let store = BlobStore::temp();
	for path in paths {
		let path = RelPath::from(*path);
		let bytes = disk.get(&path).await.unwrap();
		store.insert(&path, bytes).await.unwrap();
	}
	store
}
