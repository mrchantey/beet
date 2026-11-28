//! One sync between any two stores, the core of `<StoreSync/>` and
//! `beet store-sync`.

use crate::prelude::*;
use beet_core::prelude::*;

/// A store a sync reads or writes, as it is authored: a uri, or a declaration
/// by reference, ie `<StoreSync src="fs:site/assets" dest={$assets}/>`. Which
/// end is the source is the sync's direction.
#[derive(Debug, Clone, PartialEq, Reflect, MapEntities)]
pub enum StoreEnd {
	/// The store this uri names.
	Uri(StoreUri),
	/// The store this entity declares.
	Store(#[entities] Entity),
}

impl From<StoreUri> for StoreEnd {
	fn from(uri: StoreUri) -> Self { Self::Uri(uri) }
}

impl From<Entity> for StoreEnd {
	fn from(entity: Entity) -> Self { Self::Store(entity) }
}

/// One end of a [`StoreMirror`], a resolved store.
#[derive(Debug, Clone)]
pub struct SyncEnd {
	store: SyncStore,
	/// Where the end is, for the sync's report.
	label: SmolStr,
}

/// The stores a [`StoreMirror`] option is for, and any other.
#[derive(Debug, Clone)]
enum SyncStore {
	Fs(FsStore),
	#[cfg(feature = "aws")]
	S3(S3Store),
	Erased(BlobStore),
}

impl SyncEnd {
	/// The blob store `uri` names, a relative path resolved as
	/// [`BlobStoreProvider::from_uri`] resolves one.
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		Self {
			label: uri.to_string().into(),
			store: match BlobStoreProvider::from_uri(uri)? {
				BlobStoreProvider::Fs(store) => SyncStore::Fs(store),
				#[cfg(feature = "aws")]
				BlobStoreProvider::S3(store) => SyncStore::S3(store),
				other => SyncStore::Erased(other.into_blob_store()),
			},
		}
		.xok()
	}

	/// The blob store `target` declares, exactly the one on the entity.
	pub async fn declared(world: &AsyncWorld, target: Entity) -> Result<Self> {
		let blobs = StoreRef::resolve::<BlobStore>(world, target).await?;
		let store = world
			.entity(target)
			.with(move |entity| {
				// the concrete store where an option is for one, unless it is
				// not what the entity's `BlobStore` is
				if let Some(store) = entity.get::<FsStore>()
					&& blobs.same_scope(&BlobStore::new(store.clone()))
				{
					return SyncStore::Fs(store.clone());
				}
				#[cfg(feature = "aws")]
				if let Some(store) = entity.get::<S3Store>()
					&& blobs.same_scope(&BlobStore::new(store.clone()))
				{
					return SyncStore::S3(store.clone());
				}
				SyncStore::Erased(blobs)
			})
			.await?;
		let mut end = Self {
			label: default(),
			store,
		};
		end.label = end.blob_store().describe().into();
		end.xok()
	}

	/// This end a folder further in.
	pub fn with_dir(mut self, dir: Option<FwdPath>) -> Self {
		if let Some(dir) = dir {
			self.label = format!("{}/{dir}", self.label).into();
			self.store = match self.store {
				SyncStore::Fs(store) => SyncStore::Fs(
					store
						.clone()
						.with_subdir(BlobProvider::subdir(&store).join(&dir)),
				),
				#[cfg(feature = "aws")]
				SyncStore::S3(store) => SyncStore::S3(
					store
						.clone()
						.with_subdir(BlobProvider::subdir(&store).join(&dir)),
				),
				SyncStore::Erased(store) => {
					SyncStore::Erased(store.with_subdir(dir))
				}
			};
		}
		self
	}

	/// Write every object in this S3 storage class, where this end is one.
	#[cfg(feature = "aws")]
	pub fn with_storage_class(mut self, class: Option<&str>) -> Self {
		if let (Some(class), SyncStore::S3(store)) = (class, &self.store) {
			self.store = SyncStore::S3(store.clone().with_storage_class(class));
		}
		self
	}

	/// The store, erased.
	pub fn blob_store(&self) -> BlobStore {
		match &self.store {
			SyncStore::Fs(store) => BlobStore::new(store.clone()),
			#[cfg(feature = "aws")]
			SyncStore::S3(store) => BlobStore::new(store.clone()),
			SyncStore::Erased(store) => store.clone(),
		}
	}

	/// Change the filesystem store this end is, answering whether it is one.
	fn map_fs(&mut self, func: impl FnOnce(FsStore) -> FsStore) -> bool {
		match &self.store {
			SyncStore::Fs(store) => {
				self.store = SyncStore::Fs(func(store.clone()));
				true
			}
			_ => false,
		}
	}

	/// Read unsigned, answering whether this end is an S3 store.
	fn set_anonymous(&mut self) -> bool {
		match &self.store {
			#[cfg(feature = "aws")]
			SyncStore::S3(store) => {
				self.store = SyncStore::S3(store.clone().with_anonymous(true));
				true
			}
			_ => false,
		}
	}

	/// Address `region`, answering whether this end is an S3 store.
	#[cfg_attr(not(feature = "aws"), allow(unused_variables))]
	fn set_region(&mut self, region: &SmolStr) -> bool {
		match &self.store {
			#[cfg(feature = "aws")]
			SyncStore::S3(store) => {
				self.store =
					SyncStore::S3(store.clone().with_region(region.clone()));
				true
			}
			_ => false,
		}
	}
}

impl core::fmt::Display for SyncEnd {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.label.fmt(f)
	}
}

/// A [`BlobSync`] between two [`SyncEnd`]s, with the options that belong to
/// one kind of end: each is refused on a sync it means nothing to, rather than
/// ignored.
#[derive(Debug, Clone, SetWith)]
pub struct StoreMirror {
	#[set_with(skip)]
	src: SyncEnd,
	#[set_with(skip)]
	dest: SyncEnd,
	/// Remove objects in the destination that the source lacks.
	delete: bool,
	/// Compare the two ends by size alone, see [`BlobSync::size_only`].
	size_only: bool,
	/// Report what would move without moving it.
	dry_run: bool,
	/// The paths below both ends this sync covers, every one when empty.
	filter: GlobFilter,
	/// Read through the symbolic links of a filesystem end rather than
	/// skipping them.
	follow_symlinks: bool,
	/// Read an S3 source unsigned, for a public bucket.
	anonymous: bool,
	/// The region of the S3 ends, else the one each names or the launch's.
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
}

impl StoreMirror {
	/// An additive sync of `src` into `dest`.
	pub fn new(src: SyncEnd, dest: SyncEnd) -> Self {
		Self {
			src,
			dest,
			delete: false,
			size_only: false,
			dry_run: false,
			filter: default(),
			follow_symlinks: false,
			anonymous: false,
			region: None,
		}
	}

	/// Run the sync, reporting what moved, or under `dry_run` what would.
	pub async fn run(mut self) -> Result<BlobSyncReport> {
		let follow = self.follow_symlinks;
		let fs_end = [&mut self.src, &mut self.dest]
			.into_iter()
			.map(|end| end.map_fs(|fs| fs.with_follow_symlinks(follow)))
			.fold(false, |any, end| any || end);
		if follow && !fs_end {
			bevybail!(
				"`follow_symlinks` is for a filesystem end, and neither {} nor \
				 {} is one",
				self.src,
				self.dest
			);
		}
		if self.anonymous && !self.src.set_anonymous() {
			bevybail!(
				"`anonymous` is an unsigned read of an S3 source, and {} is not \
				 one",
				self.src
			);
		}
		if let Some(region) = self.region.clone() {
			let s3_end = [&mut self.src, &mut self.dest]
				.into_iter()
				.map(|end| end.set_region(&region))
				.fold(false, |any, end| any || end);
			if !s3_end {
				bevybail!(
					"`region` is for an S3 end, and neither {} nor {} is one",
					self.src,
					self.dest
				);
			}
		}
		if !self.filter.is_empty() {
			// a sync that silently skipped most of a store is the kind of
			// surprise worth a line of output
			info!("filtered sync: {}", self.filter);
		}
		let report =
			BlobSync::new(self.src.blob_store(), self.dest.blob_store())
				.with_delete(self.delete)
				.with_size_only(self.size_only)
				.with_dry_run(self.dry_run)
				.with_filter(core::mem::take(&mut self.filter))
				.run()
				.await?;
		info!("synced {} -> {}: {report}", self.src, self.dest);
		if let Some(warning) = self.kept_warning(&report) {
			warn!("{warning}");
		}
		report.xok()
	}

	/// The objects an additive sync left in `dest` that `src` lacks, which
	/// either end may be missing by mistake: a stale file a pull keeps (and a
	/// deploy assembling the directory then ships), or one never pushed.
	fn kept_warning(&self, report: &BlobSyncReport) -> Option<String> {
		const SHOWN: usize = 10;
		if report.kept.is_empty() {
			return None;
		}
		let mut listed = report
			.kept
			.iter()
			.take(SHOWN)
			.map(|path| path.to_string())
			.collect::<Vec<_>>()
			.join(", ");
		if report.kept.len() > SHOWN {
			listed
				.push_str(&format!(", and {} more", report.kept.len() - SHOWN));
		}
		format!(
			"{} holds {} object(s) {} does not: {listed}. Sync them the other \
			 way if they belong, or remove them; a `delete` sync prunes them",
			self.dest,
			report.kept.len(),
			self.src
		)
		.xsome()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The end of a memory store holding `entries`, kept alive beside it.
	async fn memory(name: &str, entries: &[(&str, &str)]) -> SyncEnd {
		let uri = StoreUri::parse(&format!("memory://{name}")).unwrap();
		let end = SyncEnd::from_uri(&uri).unwrap();
		for (path, body) in entries {
			end.blob_store()
				.insert(&FwdPath::new(path).unwrap(), body.to_string())
				.await
				.unwrap();
		}
		end
	}

	async fn listed(end: &SyncEnd) -> Vec<String> {
		let mut paths = end.blob_store().list().await.unwrap();
		paths.sort();
		paths.into_iter().map(|path| path.to_string()).collect()
	}

	/// What an additive sync leaves behind is named, ends and all, so a stale
	/// file a pull keeps is seen before a deploy assembles it.
	#[beet_core::test]
	async fn names_what_an_additive_sync_keeps() {
		let src = memory("sync-kept-src", &[("a.txt", "a")]).await;
		let dest =
			memory("sync-kept-dest", &[("a.txt", "a"), ("stale.wasm", "s")])
				.await;
		let mirror = StoreMirror::new(src.clone(), dest.clone());
		let report = mirror.clone().run().await.unwrap();
		mirror
			.kept_warning(&report)
			.unwrap()
			.xpect_contains("memory://sync-kept-dest holds 1 object(s) memory://sync-kept-src does not: stale.wasm");
		// a mirror prunes rather than keeps
		let report = mirror.with_delete(true).run().await.unwrap();
		report.kept.is_empty().xpect_true();
		listed(&dest).await.xpect_eq(vec!["a.txt".to_string()]);
	}

	/// A folder below either end is the sync's own root.
	#[beet_core::test]
	async fn syncs_below_either_end() {
		let src = memory("sync-dir-src", &[("a/x.txt", "x"), ("b.txt", "b")])
			.await
			.with_dir(Some(fwd_path!("a")));
		let dest = memory("sync-dir-dest", &[("keep.txt", "k")]).await;
		StoreMirror::new(
			src,
			dest.clone().with_dir(Some(fwd_path!("in/to"))),
		)
		.with_delete(true)
		.run()
		.await
		.unwrap();
		listed(&dest)
			.await
			.xpect_eq(vec!["in/to/x.txt", "keep.txt"]);
	}

	/// Links are skipped unless followed, and following them is refused where
	/// there are none.
	#[cfg(all(unix, not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn links_are_followed_when_asked() {
		let root = TempDir::new_ws().unwrap();
		let target = TempDir::new_ws().unwrap();
		fs_ext::write(root.join("main.bsx"), "<div/>").unwrap();
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		std::os::unix::fs::symlink(target.path(), root.join("assets")).unwrap();
		let src = SyncEnd::from_uri(&StoreUri::Fs {
			path_prefix: Some(root.path().as_str().into()),
		})
		.unwrap();
		let dest = memory("sync-links", &[]).await;
		let sync = StoreMirror::new(src.clone(), dest.clone());
		sync.clone().run().await.unwrap();
		listed(&dest).await.xpect_eq(vec!["main.bsx"]);
		sync.with_follow_symlinks(true).run().await.unwrap();
		listed(&dest)
			.await
			.xpect_eq(vec!["assets/logo.png", "main.bsx"]);
		StoreMirror::new(dest.clone(), dest.clone())
			.with_follow_symlinks(true)
			.run()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`follow_symlinks` is for a filesystem end")
			.xpect_contains("memory://sync-links");
	}

	/// An option is refused on a sync it means nothing to.
	#[beet_core::test]
	async fn refuses_an_option_for_another_end() {
		let end = memory("sync-options", &[]).await;
		let sync = StoreMirror::new(end.clone(), end);
		sync.clone()
			.with_anonymous(true)
			.run()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`anonymous` is an unsigned read of an S3 source");
		sync.with_region("us-west-2")
			.run()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`region` is for an S3 end");
	}

	/// The report names each end as it was given.
	#[cfg(feature = "aws")]
	#[beet_core::test]
	fn names_its_end() {
		SyncEnd::from_uri(&StoreUri::parse("s3://b/p?region=r").unwrap())
			.unwrap()
			.with_dir(Some(fwd_path!("in")))
			.to_string()
			.xpect_eq("s3://b/p?region=r/in");
	}
}
