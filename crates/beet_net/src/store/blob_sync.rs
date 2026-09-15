//! A mirror of one store into another, by content.
use crate::prelude::*;
use beet_core::prelude::*;

/// `BlobSync::new(source, dest).with_delete(true).run().await?`: make `dest`
/// hold what `source` holds, copying only what differs.
///
/// Store-agnostic, so the same mirror publishes a staged directory into an S3
/// prefix, a memory store or a directory, and a test compares the two ends
/// directly. Both ends are read through
/// [`list_stats`](BlobStoreProvider::list_stats), so an object whose size and
/// digest already match is never re-sent: on S3 one listing answers every
/// stat, which is what keeps a content-only sync of a large tree quick.
///
/// The defaults are the conservative ones: additive. `delete` opts into a true
/// mirror, where objects absent from the source are removed so the destination
/// exactly reflects it. A deploy wants that (a renamed or removed source file
/// otherwise lingers across deploys), a source of record does not.
#[derive(Debug, Clone, Get, SetWith)]
pub struct BlobSync {
	/// The end read from.
	source: BlobStore,
	/// The end written to.
	dest: BlobStore,
	/// Remove objects in the destination that the source lacks.
	delete: bool,
	/// Transfers in flight at once.
	concurrency: usize,
}

impl BlobSync {
	/// A mirror of `source` into `dest`, additive with 16 transfers in flight.
	pub fn new(source: BlobStore, dest: BlobStore) -> Self {
		Self {
			source,
			dest,
			delete: false,
			concurrency: 16,
		}
	}

	/// Run the mirror, reporting what moved.
	pub async fn run(&self) -> Result<BlobSyncReport> {
		let source = self.source.list_stats().await?;
		let dest = self
			.dest
			.list_stats()
			.await?
			.into_iter()
			.collect::<HashMap<_, _>>();
		// what to copy: absent from the destination, or present but different
		let mut report = BlobSyncReport::default();
		for (path, stat) in &source {
			match dest.get(path) {
				Some(existing) if existing.matches(stat) => {
					report.unchanged.push(path.clone());
					report.unchanged_bytes += stat.size;
				}
				_ => {
					report.copied.push(path.clone());
					report.copied_bytes += stat.size;
				}
			}
		}
		async_ext::try_join_all_bounded(
			self.concurrency,
			report.copied.iter().map(|path| async move {
				let bytes = self.source.get(path).await?;
				self.dest.insert(path, bytes).await
			}),
		)
		.await?;
		// what to prune: in the destination and nowhere in the source
		if self.delete {
			let kept =
				source.iter().map(|(path, _)| path).collect::<HashSet<_>>();
			report.removed = dest
				.keys()
				.filter(|path| !kept.contains(path))
				.cloned()
				.collect();
			async_ext::try_join_all_bounded(
				self.concurrency,
				report
					.removed
					.iter()
					.map(|path| async move { self.dest.remove(path).await }),
			)
			.await?;
		}
		report.sort();
		report.xok()
	}
}

/// What one [`BlobSync::run`] moved, each list sorted so a report reads the
/// same whatever order the stores listed in.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlobSyncReport {
	/// Objects sent to the destination, new or changed.
	pub copied: Vec<RelPath>,
	/// Objects already matching at the destination, left alone.
	pub unchanged: Vec<RelPath>,
	/// Objects removed from the destination, only under `delete`.
	pub removed: Vec<RelPath>,
	/// Total size of the copied objects.
	pub copied_bytes: u64,
	/// Total size of the unchanged objects.
	pub unchanged_bytes: u64,
}

impl BlobSyncReport {
	fn sort(&mut self) {
		self.copied.sort();
		self.unchanged.sort();
		self.removed.sort();
	}

	/// A byte total as MiB with one decimal, ie `34.5 MiB`.
	fn mib(bytes: u64) -> String {
		format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
	}
}

impl core::fmt::Display for BlobSyncReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"copied {} ({}), unchanged {} ({}), removed {}",
			self.copied.len(),
			Self::mib(self.copied_bytes),
			self.unchanged.len(),
			Self::mib(self.unchanged_bytes),
			self.removed.len()
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	async fn seeded(entries: &[(&str, &str)]) -> BlobStore {
		let store = BlobStore::temp();
		for (path, body) in entries {
			store
				.insert(&RelPath::new(path), body.to_string())
				.await
				.unwrap();
		}
		store
	}

	fn paths(paths: &[&str]) -> Vec<RelPath> {
		paths.iter().map(RelPath::new).collect()
	}

	/// A fresh destination receives everything, and a second run of the same
	/// source sends nothing: the stats already match.
	#[beet_core::test]
	async fn copies_once_then_skips() {
		let source = seeded(&[("a.txt", "a"), ("dir/b.txt", "b")]).await;
		let dest = BlobStore::temp();
		let sync = BlobSync::new(source.clone(), dest.clone());
		let report = sync.run().await.unwrap();
		report.copied.xpect_eq(paths(&["a.txt", "dir/b.txt"]));
		report.unchanged.xpect_eq(Vec::<RelPath>::new());
		// one byte per seeded body, so the totals are the counts
		report.copied_bytes.xpect_eq(2);
		report.unchanged_bytes.xpect_eq(0);
		dest.get(&RelPath::new("dir/b.txt"))
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from_static(b"b"));

		let report = sync.run().await.unwrap();
		report.copied.xpect_eq(Vec::<RelPath>::new());
		report.unchanged.xpect_eq(paths(&["a.txt", "dir/b.txt"]));
		report.copied_bytes.xpect_eq(0);
		report.unchanged_bytes.xpect_eq(2);
	}

	/// A changed object is re-sent even at the same size: the digest decides,
	/// not the length.
	#[beet_core::test]
	async fn resends_a_changed_object() {
		let source = seeded(&[("a.txt", "one"), ("b.txt", "two")]).await;
		let dest = seeded(&[("a.txt", "one"), ("b.txt", "TWO")]).await;
		let report = BlobSync::new(source, dest.clone()).run().await.unwrap();
		report.copied.xpect_eq(paths(&["b.txt"]));
		report.unchanged.xpect_eq(paths(&["a.txt"]));
		dest.get(&RelPath::new("b.txt"))
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from_static(b"two"));
	}

	/// An object the source lacks survives an additive sync and goes under
	/// `delete`, so only a mirror can destroy destination state.
	#[beet_core::test]
	async fn delete_prunes_only_when_asked() {
		let source = seeded(&[("keep.txt", "k")]).await;
		let dest = seeded(&[("keep.txt", "k"), ("stale.txt", "s")]).await;
		let sync = BlobSync::new(source, dest.clone());
		sync.run()
			.await
			.unwrap()
			.removed
			.xpect_eq(Vec::<RelPath>::new());
		dest.exists(&RelPath::new("stale.txt"))
			.await
			.unwrap()
			.xpect_true();

		let report = sync.clone().with_delete(true).run().await.unwrap();
		report.removed.xpect_eq(paths(&["stale.txt"]));
		report.unchanged.xpect_eq(paths(&["keep.txt"]));
		dest.list().await.unwrap().xpect_eq(paths(&["keep.txt"]));
	}

	/// The deploy's shape: a directory on disk mirrored into a store, and the
	/// two ends compared afterwards.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn mirrors_a_directory() {
		let root =
			AbsPath::new_workspace_rel("target/tests/beet_net/blob-sync")
				.unwrap();
		fs_ext::remove(&root).ok();
		fs_ext::write(root.join("main.bsx"), "<div/>").unwrap();
		fs_ext::write(root.join("assets/logo.png"), "png").unwrap();
		let source = BlobStore::new(FsStore::new(root));
		let dest = BlobStore::temp();
		BlobSync::new(source.clone(), dest.clone())
			.with_delete(true)
			.run()
			.await
			.unwrap()
			.copied
			.xpect_eq(paths(&["assets/logo.png", "main.bsx"]));
		let mut stats = dest.list_stats().await.unwrap();
		stats.sort_by(|a, b| a.0.cmp(&b.0));
		let mut expected = source.list_stats().await.unwrap();
		expected.sort_by(|a, b| a.0.cmp(&b.0));
		stats.xpect_eq(expected);
	}
}
