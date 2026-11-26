//! A mirror of one store into another, by content.
use crate::prelude::*;
use beet_core::prelude::*;

/// `BlobSync::new(source, dest).with_delete(true).run().await?`: make `dest`
/// hold what `source` holds, copying only what differs.
///
/// Store-agnostic, so the same mirror publishes a staged directory into an S3
/// prefix, a memory store or a directory, and a test compares the two ends
/// directly. Both ends are read through
/// [`list_stats`](BlobProvider::list_stats), so an object whose size and
/// digest already match is never re-sent: on S3 one listing answers every
/// stat, which is what keeps a content-only sync of a large tree quick.
///
/// The defaults are the conservative ones: additive. `delete` opts into a true
/// mirror, where objects absent from the source are removed so the destination
/// exactly reflects it. A deploy wants that (a renamed or removed source file
/// otherwise lingers across deploys), a source of record does not. A mirror is
/// refused where its source is empty or missing, or reads through a link to an
/// empty directory ([`BlobProvider::empty_links`]): the signature of an
/// unhydrated checkout, which would empty the destination.
///
/// What each end is, beyond its contents, is the end's own declaration: a
/// local end that skips symbolic links is an
/// [`FsStore::with_follow_symlinks`], a storage class or an unsigned read of
/// a public bucket an [`S3Store`](crate::prelude::S3Store) option, a prefix a
/// subdir.
#[derive(Debug, Clone, Get, SetWith)]
pub struct BlobSync {
	/// The end read from.
	source: BlobStore,
	/// The end written to.
	dest: BlobStore,
	/// Remove objects in the destination that the source lacks.
	delete: bool,
	/// Report what would move without moving it.
	dry_run: bool,
	/// Compare the two ends by size alone, rather than size and digest.
	///
	/// Right for a store whose content is immutable per path (a segment log,
	/// a content-addressed blob), where size is already a complete
	/// comparison. Wrong for a mutable mirror, where an edit that happened to
	/// keep the byte count would be skipped.
	size_only: bool,
	/// The paths below both ends this sync covers, the whole store when
	/// empty: a path the filter refuses is neither copied nor removed, so a
	/// pattern reads the same in either direction.
	filter: GlobFilter,
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
			dry_run: false,
			size_only: false,
			filter: default(),
			concurrency: 16,
		}
	}

	/// The stats of `store` this sync covers.
	async fn covered(
		&self,
		store: &BlobStore,
	) -> Result<Vec<(RelPath, BlobStat)>> {
		store
			.list_stats()
			.await?
			.into_iter()
			.filter(|(path, _)| {
				self.filter.is_empty() || self.filter.passes(path.as_str())
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Whether `dest` already holds what `source` describes.
	fn same(&self, dest: &BlobStat, source: &BlobStat) -> bool {
		match self.size_only {
			true => dest.size == source.size,
			false => dest.matches(source),
		}
	}

	/// Refuse to mirror a source that would empty the destination.
	async fn assert_mirrorable(&self) -> Result {
		if self.source.store_is_empty().await? {
			bevybail!(
				"refusing to mirror an empty store into {}: {} is missing or \
				 empty, hydrate it first",
				self.dest.describe(),
				self.source.describe()
			);
		}
		if let Some(link) = self.source.empty_links().await?.first() {
			bevybail!(
				"refusing to mirror {} into {}: its link `{link}` names an \
				 empty directory, hydrate it first",
				self.source.describe(),
				self.dest.describe()
			);
		}
		Ok(())
	}

	/// Run the mirror, reporting what moved, or under
	/// [`dry_run`](Self::dry_run) what would.
	pub async fn run(&self) -> Result<BlobSyncReport> {
		// only a mirror can destroy destination state, so only it is guarded
		if self.delete {
			self.assert_mirrorable().await?;
		}
		let source = self.covered(&self.source).await?;
		let dest = self
			.covered(&self.dest)
			.await?
			.into_iter()
			.collect::<HashMap<_, _>>();
		// what to copy: absent from the destination, or present but different
		let mut report = BlobSyncReport {
			dry_run: self.dry_run,
			..default()
		};
		for (path, stat) in &source {
			match dest.get(path) {
				Some(existing) if self.same(existing, stat) => {
					report.unchanged.push(path.clone());
					report.unchanged_bytes += stat.size;
				}
				_ => {
					report.copied.push(path.clone());
					report.copied_bytes += stat.size;
				}
			}
		}
		if !self.dry_run {
			async_ext::try_join_all_bounded(
				self.concurrency,
				report.copied.iter().map(|path| async move {
					let bytes = self.source.get(path).await?;
					self.dest.insert(path, bytes).await
				}),
			)
			.await?;
		}
		// in the destination and nowhere in the source: pruned by a mirror,
		// else left in place and reported
		let in_source =
			source.iter().map(|(path, _)| path).collect::<HashSet<_>>();
		let extra = dest
			.keys()
			.filter(|path| !in_source.contains(path))
			.cloned()
			.collect::<Vec<_>>();
		match self.delete {
			false => report.kept = extra,
			true => report.removed = extra,
		}
		if self.delete {
			if !self.dry_run {
				async_ext::try_join_all_bounded(
					self.concurrency,
					report.removed.iter().map(|path| async move {
						self.dest.remove(path).await
					}),
				)
				.await?;
			}
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
	/// Objects the destination holds and the source lacks, left in place by
	/// an additive sync: a file never pushed, or a stale one a pull keeps.
	pub kept: Vec<RelPath>,
	/// Total size of the copied objects.
	pub copied_bytes: u64,
	/// Total size of the unchanged objects.
	pub unchanged_bytes: u64,
	/// Whether this was a dry run, so nothing listed above moved.
	pub dry_run: bool,
}

impl BlobSyncReport {
	fn sort(&mut self) {
		self.copied.sort();
		self.unchanged.sort();
		self.removed.sort();
		self.kept.sort();
	}

	/// A byte total as MiB with one decimal, ie `34.5 MiB`.
	fn mib(bytes: u64) -> String {
		format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
	}
}

impl core::fmt::Display for BlobSyncReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let would = match self.dry_run {
			true => "would have ",
			false => "",
		};
		write!(
			f,
			"{would}copied {} ({}), unchanged {} ({}), removed {}",
			self.copied.len(),
			Self::mib(self.copied_bytes),
			self.unchanged.len(),
			Self::mib(self.unchanged_bytes),
			self.removed.len()
		)?;
		match self.kept.len() {
			0 => Ok(()),
			kept => write!(f, ", kept {kept} the source lacks"),
		}
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

	/// An object the source lacks survives an additive sync, reported as
	/// kept, and goes under `delete`, so only a mirror can destroy destination
	/// state.
	#[beet_core::test]
	async fn delete_prunes_only_when_asked() {
		let source = seeded(&[("keep.txt", "k")]).await;
		let dest = seeded(&[("keep.txt", "k"), ("stale.txt", "s")]).await;
		let sync = BlobSync::new(source, dest.clone());
		let report = sync.run().await.unwrap();
		report.removed.xpect_eq(Vec::<RelPath>::new());
		report.kept.xpect_eq(paths(&["stale.txt"]));
		report
			.to_string()
			.xpect_contains("kept 1 the source lacks");
		dest.exists(&RelPath::new("stale.txt"))
			.await
			.unwrap()
			.xpect_true();

		let report = sync.clone().with_delete(true).run().await.unwrap();
		report.removed.xpect_eq(paths(&["stale.txt"]));
		report.kept.xpect_eq(Vec::<RelPath>::new());
		report.unchanged.xpect_eq(paths(&["keep.txt"]));
		dest.list().await.unwrap().xpect_eq(paths(&["keep.txt"]));
	}

	/// A mirror of an empty or missing source is refused, naming it, whatever
	/// the store; an additive sync of one moves nothing.
	#[beet_core::test]
	async fn refuses_to_mirror_an_empty_source() {
		let dest = seeded(&[("keep.txt", "k")]).await;
		let source = BlobStore::temp();
		let sync = BlobSync::new(source.clone(), dest.clone());
		sync.run().await.unwrap().copied.xpect_empty();
		for dry_run in [false, true] {
			sync.clone()
				.with_delete(true)
				.with_dry_run(dry_run)
				.run()
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("refusing to mirror")
				.xpect_contains(&source.describe());
		}
		#[cfg(not(target_arch = "wasm32"))]
		{
			let dir = TempDir::new_ws().unwrap();
			for missing in [dir.path().clone(), dir.join("missing")] {
				BlobSync::new(
					BlobStore::new(FsStore::new(missing.clone())),
					dest.clone(),
				)
				.with_delete(true)
				.run()
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains(&missing.to_string());
			}
		}
		dest.list().await.unwrap().xpect_eq(paths(&["keep.txt"]));
	}

	/// A mirror reading through a link to an empty or missing directory is
	/// refused, at any depth, and only where links are followed.
	#[cfg(all(unix, not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn refuses_to_mirror_an_unhydrated_link() {
		let root = TempDir::new_ws().unwrap();
		let target = TempDir::new_ws().unwrap();
		fs_ext::write(root.join("main.bsx"), "<div/>").unwrap();
		fs_ext::create_dir_all(root.join("nested")).unwrap();
		std::os::unix::fs::symlink(target.path(), root.join("nested/assets"))
			.unwrap();
		let mirror = |follow: bool| {
			BlobSync::new(
				BlobStore::new(
					FsStore::new(root.path().clone())
						.with_follow_symlinks(follow),
				),
				BlobStore::temp(),
			)
			.with_delete(true)
		};
		mirror(false).run().await.unwrap();
		mirror(true)
			.run()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("nested/assets")
			.xpect_contains(root.path().as_str());
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		mirror(true).run().await.unwrap();
		fs_ext::remove(target.path()).unwrap();
		mirror(true).run().await.unwrap_err();
	}

	/// A dry run reports what a mirror would do and touches nothing.
	#[beet_core::test]
	async fn a_dry_run_moves_nothing() {
		let source = seeded(&[("new.txt", "n")]).await;
		let dest = seeded(&[("stale.txt", "s")]).await;
		let report = BlobSync::new(source, dest.clone())
			.with_delete(true)
			.with_dry_run(true)
			.run()
			.await
			.unwrap();
		report.copied.xpect_eq(paths(&["new.txt"]));
		report.removed.xpect_eq(paths(&["stale.txt"]));
		report.to_string().xpect_starts_with("would have copied 1");
		dest.list().await.unwrap().xpect_eq(paths(&["stale.txt"]));
	}

	/// By size alone, a same-length change is not re-sent.
	#[beet_core::test]
	async fn size_only_skips_a_same_size_change() {
		let source = seeded(&[("a.txt", "one")]).await;
		let dest = seeded(&[("a.txt", "ONE")]).await;
		BlobSync::new(source, dest)
			.with_size_only(true)
			.run()
			.await
			.unwrap()
			.unchanged
			.xpect_eq(paths(&["a.txt"]));
	}

	/// A filter narrows both ends, so what it refuses is neither copied nor
	/// pruned.
	#[beet_core::test]
	async fn a_filter_covers_both_ends() {
		let source =
			seeded(&[("records/a.json", "a"), ("blobs/b.png", "b")]).await;
		let dest = seeded(&[("blobs/kept.png", "k")]).await;
		let report = BlobSync::new(source, dest.clone())
			.with_delete(true)
			.with_filter(GlobFilter::default().with_exclude("blobs/*"))
			.run()
			.await
			.unwrap();
		report.copied.xpect_eq(paths(&["records/a.json"]));
		report.removed.xpect_eq(Vec::<RelPath>::new());
		dest.exists(&RelPath::new("blobs/kept.png"))
			.await
			.unwrap()
			.xpect_true();
	}

	/// A local end that skips links leaves a linked directory out.
	#[cfg(all(unix, not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn a_local_end_may_skip_links() {
		let root = TempDir::new_ws().unwrap();
		let target = TempDir::new_ws().unwrap();
		fs_ext::write(root.join("main.bsx"), "<div/>").unwrap();
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		std::os::unix::fs::symlink(target.path(), root.join("assets")).unwrap();
		let sync = |follow: bool| {
			BlobSync::new(
				BlobStore::new(
					FsStore::new(root.path().clone())
						.with_follow_symlinks(follow),
				),
				BlobStore::temp(),
			)
		};
		sync(true)
			.run()
			.await
			.unwrap()
			.copied
			.xpect_eq(paths(&["assets/logo.png", "main.bsx"]));
		sync(false)
			.run()
			.await
			.unwrap()
			.copied
			.xpect_eq(paths(&["main.bsx"]));
	}

	/// The deploy's shape: a directory on disk mirrored into a store, and the
	/// two ends compared afterwards.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn mirrors_a_directory() {
		let root = TempDir::new_ws().unwrap();
		fs_ext::write(root.join("main.bsx"), "<div/>").unwrap();
		fs_ext::write(root.join("assets/logo.png"), "png").unwrap();
		let source = BlobStore::new(FsStore::new(root.path().clone()));
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
