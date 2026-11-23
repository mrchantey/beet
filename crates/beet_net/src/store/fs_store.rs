use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Filesystem-backed store for local storage.
///
/// Stores objects as files on the local filesystem, with the configured
/// path representing the full store directory.
///
/// ## Default
/// The default store is relative to the workspace root.
#[derive(Debug, Clone, Component, Reflect, Get)]
#[reflect(Component, Default)]
#[component(on_insert = BlobStore::on_insert::<Self>)]
pub struct FsStore {
	/// The full path to the store directory. Coerces from a workspace-relative
	/// string attribute in markup, ie `<FsStore path="assets"/>`.
	path: AbsPath,
	/// Optional subdirectory from which all paths are resolved.
	subdir: Option<RelPath>,
	/// Whether a listing reads through symbolic links, materializing a linked
	/// directory's files as the store's own. On by default, as a path read
	/// is; off, a link is invisible to [`list`](BlobStoreProvider::list).
	follow_symlinks: bool,
}

impl Default for FsStore {
	fn default() -> Self {
		Self {
			path: WsPath::default().into(),
			subdir: None,
			follow_symlinks: true,
		}
	}
}

impl FsStore {
	/// Create a new filesystem store with the given store path.
	pub fn new(path: impl Into<AbsPath>) -> Self {
		Self {
			path: path.into(),
			..default()
		}
	}

	/// List through symbolic links or skip them, see
	/// [`FsStore::follow_symlinks`].
	pub fn with_follow_symlinks(mut self, follow_symlinks: bool) -> Self {
		self.follow_symlinks = follow_symlinks;
		self
	}
	/// Set the subdirectory from which all paths are resolved.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}
	/// Resolve the effective root directory, including subdir if set.
	pub fn effective_root(&self) -> AbsPath {
		match &self.subdir {
			Some(sub) => self.path.join(sub),
			None => self.path.clone(),
		}
	}
	/// Resolve the full path for an object key.
	fn resolve_path(&self, route: &RelPath) -> AbsPath {
		self.effective_root().join(route)
	}
}

/// The locks a conditional write holds while it checks then writes a file,
/// striped by path so unrelated files never wait on each other. In process
/// only: two processes writing one file conditionally are not serialized.
static CONDITIONAL_LOCKS: [async_lock::Mutex<()>; 64] =
	[const { async_lock::Mutex::new(()) }; 64];

impl FsStore {
	/// The stripe of [`CONDITIONAL_LOCKS`] guarding `path`.
	fn conditional_lock(path: &AbsPath) -> &'static async_lock::Mutex<()> {
		use core::hash::BuildHasher;
		let hash = bevy::platform::hash::FixedHasher.hash_one(path.as_str());
		&CONDITIONAL_LOCKS[hash as usize % CONDITIONAL_LOCKS.len()]
	}

	/// The file at `path` with its digest, `None` when there is none.
	async fn read_versioned(
		path: &AbsPath,
	) -> Result<Option<(Bytes, BlobVersion)>> {
		if !fs_ext::exists_async(path).await? {
			return Ok(None);
		}
		let bytes = Bytes::from(fs_ext::read_async(path).await?);
		let version = BlobVersion::of(&bytes);
		Some((bytes, version)).xok()
	}
}

impl BlobStoreProvider for FsStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(FsStore {
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
			..self.clone()
		})
	}

	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(FsStore {
			subdir: None,
			..self.clone()
		})
	}

	/// The filesystem has a parent universe above the store's root, so a
	/// `../..` root re-roots the store at the absolute resolved directory
	/// rather than erroring — walking above the entry's directory is the
	/// point of an fs `<RepoRoot>`.
	fn rebase(
		&self,
		entry_name: &RelPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobStoreProvider>, RelPath)> {
		let abs_root = self.effective_root().join(root);
		let entry_name = self
			.effective_root()
			.join(entry_name)
			.strip_prefix(&abs_root)
			.ok_or_else(|| {
				bevyhow!(
					"entry `{entry_name}` is not under its declared store root \
					`{abs_root}`"
				)
			})?;
		(
			(Box::new(FsStore {
				path: abs_root,
				subdir: None,
				..self.clone()
			}) as Box<dyn BlobStoreProvider>),
			entry_name,
		)
			.xok()
	}

	fn id(&self) -> &'static str { "fs" }

	fn root_key(&self) -> SmolStr { format!("fs:{}", self.path).into() }

	fn subdir(&self) -> RelPath { self.subdir.clone().unwrap_or_default() }

	fn watch_dir(&self) -> Option<AbsPath> { Some(self.effective_root()) }

	fn base_dir(&self) -> Option<AbsPath> { Some(self.path.clone()) }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let root = self.effective_root();
		Box::pin(async move { fs_ext::exists_async(root).await?.xok() })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let root = self.effective_root();
		Box::pin(async move {
			fs_ext::create_dir_all_async(root).await?;
			().xok()
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let root = self.effective_root();
		Box::pin(async move {
			fs_ext::remove_async(root).await?;
			().xok()
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			fs_ext::write_async(path, body).await?;
			().xok()
		})
	}

	/// A copy on the blocking pool, so a multi-gigabyte object never passes
	/// through memory (and is a reflink where the filesystem has them).
	fn insert_file(
		&self,
		path: &RelPath,
		file: &AbsPath,
	) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		let file = file.clone();
		Box::pin(async move {
			cfg_if! {
				if #[cfg(all(feature = "fs", not(target_arch = "wasm32")))] {
					async_ext::unblock(move || fs_ext::copy(&file, &path)).await?;
				} else {
					fs_ext::copy(&file, &path)?;
				}
			}
			().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let root = self.effective_root();
		let scoped = self.subdir.is_some();
		let read = ReadDir {
			files: true,
			recursive: true,
			follow_symlinks: self.follow_symlinks,
			..default()
		};
		Box::pin(async move {
			// a subdir never written to holds no files, as a missing prefix in
			// an object store does; only a missing root is an error
			if scoped && !fs_ext::exists_async(&root).await? {
				return Vec::new().xok();
			}
			read.read_async(&root)
				.await?
				.into_iter()
				// a conditional write's file on its way into place
				.filter(|path| {
					!path.to_string_lossy().ends_with(fs_ext::REPLACING_SUFFIX)
				})
				.map(|path| {
					let path = AbsPath::new_unchecked(path.to_string_lossy());
					path.strip_prefix(&root)
						.unwrap_or_else(|| RelPath::new(path))
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// One `read_dir`, a missing directory empty.
	fn list_dir(&self, path: &RelPath) -> SendBoxedFuture<Result<BlobDir>> {
		let dir = self.resolve_path(path);
		Box::pin(async move {
			if !fs_ext::exists_async(&dir).await? {
				return BlobDir::default().xok();
			}
			let name_of = |path: std::path::PathBuf| {
				path.file_name()
					.map(|name| SmolStr::new(name.to_string_lossy()))
			};
			BlobDir {
				dirs: ReadDir::dirs_async(&dir)
					.await?
					.into_iter()
					.filter_map(name_of)
					.collect(),
				files: ReadDir::files_async(&dir)
					.await?
					.into_iter()
					.filter_map(name_of)
					.collect(),
			}
			.dedup()
			.xok()
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			fs_ext::read_async(&path)
				.await
				.map_err(|_| HttpError::not_found())?
				.xmap(Bytes::from)
				.xok()
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::exists_async(path).await?.xok() })
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::remove_async(path).await?.xok() })
	}

	fn get_versioned(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		let path = self.resolve_path(path);
		Box::pin(async move { Self::read_versioned(&path).await })
	}

	/// Checked and written under the path's in-process lock, the version
	/// the content's digest, the file replaced whole so an unlocked reader
	/// never reads it torn.
	fn insert_if(
		&self,
		path: &RelPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		let rel = path.clone();
		let path = self.resolve_path(path);
		Box::pin(async move {
			let _guard = Self::conditional_lock(&path).lock().await;
			let found = Self::read_versioned(&path).await?;
			BlobConflict::check(
				&condition,
				&rel,
				found.as_ref().map(|(_, version)| version),
			)?;
			let version = BlobVersion::of(&body);
			fs_ext::write_replacing_async(&path, body).await?;
			version.xok()
		})
	}

	fn remove_if(
		&self,
		path: &RelPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		let rel = path.clone();
		let path = self.resolve_path(path);
		Box::pin(async move {
			let _guard = Self::conditional_lock(&path).lock().await;
			let found = Self::read_versioned(&path).await?;
			BlobConflict::check(
				&condition,
				&rel,
				found.as_ref().map(|(_, version)| version),
			)?;
			if found.is_some() {
				fs_ext::remove_async(&path).await?;
			}
			Ok(())
		})
	}

	/// Digested in chunks on the blocking pool, so a stat of a multi-gigabyte
	/// file never holds it.
	fn stat(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<BlobStat>>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			if !fs_ext::exists_async(&path).await? {
				return Ok(None);
			}
			cfg_if! {
				if #[cfg(all(feature = "fs", not(target_arch = "wasm32")))] {
					async_ext::unblock(move || -> Result<BlobStat> {
						BlobStat {
							size: fs_ext::file_size(&path)?,
							md5: Some(
								digest_ext::hex_file::<md5::Md5>(&path)?.into(),
							),
						}
						.xok()
					})
					.await?
					.xsome()
					.xok()
				} else {
					BlobStat::of(&fs_ext::read(&path)?).xsome().xok()
				}
			}
		})
	}

	/// From metadata, without reading the file.
	fn size(&self, path: &RelPath) -> SendBoxedFuture<Result<Option<u64>>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			if !fs_ext::exists_async(&path).await? {
				return Ok(None);
			}
			fs_ext::file_size(&path)?.xsome().xok()
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	// cross-platform: `FsStore` reads/writes through `fs_ext`, which routes to the
	// deno runner's fs globals on wasm, so the same suite runs under both. On wasm
	// the runner supplies `BEET_WORKSPACE_ROOT` (resolving the workspace-relative
	// `TempDir`) and `--allow-write`.
	#[beet_core::test]
	async fn works() {
		let dir = TempDir::new_ws().unwrap();
		store_test::run(FsStore::new(dir.path().clone())).await;
	}

	/// A subdir never written to lists empty, as a missing prefix in an object
	/// store does, so a table that has no rows yet reads as empty.
	#[beet_core::test]
	async fn lists_a_missing_subdir_as_empty() {
		let dir = TempDir::new_ws().unwrap();
		BlobStore::new(FsStore::new(dir.path().clone()))
			.with_subdir(RelPath::new("never/written"))
			.list()
			.await
			.unwrap()
			.xpect_eq(Vec::<RelPath>::new());
	}

	/// The filesystem's parent universe lets a rebase walk above the store's
	/// current root: the store re-roots at the absolute resolved directory and
	/// the entry name grows the path back down to it.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn rebases_above_the_repo_root() {
		let tmp = TempDir::new_ws().unwrap();
		let entry_dir = tmp.join("a/b");
		fs_ext::create_dir_all(&entry_dir).unwrap();
		fs_ext::write(entry_dir.join("main.bsx"), "<Router/>").unwrap();
		fs_ext::write(tmp.join("shared.txt"), "shared").unwrap();
		let store = BlobStore::new(FsStore::new(entry_dir));
		let (rebased, entry_name) =
			store.rebase_repo("main.bsx", "../..").unwrap();
		entry_name.xpect_eq("a/b/main.bsx");
		rebased
			.get_media(&RelPath::from("a/b/main.bsx"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("<Router/>");
		rebased
			.get_media(&RelPath::from("shared.txt"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("shared");
	}
}
