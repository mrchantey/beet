use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Filesystem-backed store for local storage.
///
/// Stores objects as files on the local filesystem, with the configured
/// path representing the full store directory.
///
/// ## Conditional writes
/// A conditional write holds against every other writer of the directory, in
/// this process and any other: two processes rewriting one file never erase
/// each other's changes. Between processes it is an advisory lock on the
/// directory, which the kernel releases however a process ends, so it holds
/// against other conditional writes alone, as an unconditional one takes no
/// lock. It is in process alone where a directory cannot be opened to lock
/// (Windows, wasm), and may be on a network filesystem.
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
	subdir: Option<FwdPath>,
	/// Whether a listing reads through symbolic links, materializing a linked
	/// directory's files as the store's own. On by default, as a path read
	/// is; off, a link is invisible to [`list`](BlobProvider::list).
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
	/// The key of `path` in the store rooted at `root`, refusing a path
	/// outside it, ie a followed link's own location.
	fn key_of(root: &AbsPath, path: &AbsPath) -> Result<FwdPath> {
		path.strip_prefix(root).ok_or_else(|| {
			bevyhow!("`{path}` is listed outside the store root `{root}`")
		})
	}

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
	pub fn with_subdir(mut self, subdir: impl Into<FwdPath>) -> Self {
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
	fn resolve_path(&self, route: &FwdPath) -> AbsPath {
		self.effective_root().join(route)
	}
}

/// The locks a conditional write holds while it checks then writes a file,
/// striped by its directory so unrelated directories never wait on each other.
/// Another process is held off by the directory's own lock
/// ([`fs_ext::lock_dir`]), taken inside this one.
static CONDITIONAL_LOCKS: [async_lock::Mutex<()>; 64] =
	[const { async_lock::Mutex::new(()) }; 64];

impl FsStore {
	/// The stripe of [`CONDITIONAL_LOCKS`] guarding `dir`.
	fn conditional_lock(dir: &AbsPath) -> &'static async_lock::Mutex<()> {
		use core::hash::BuildHasher;
		let hash = bevy::platform::hash::FixedHasher.hash_one(dir.as_str());
		&CONDITIONAL_LOCKS[hash as usize % CONDITIONAL_LOCKS.len()]
	}

	/// The file at `path` with its digest, `None` when there is none.
	fn read_versioned(path: &AbsPath) -> Result<Option<(Bytes, BlobVersion)>> {
		if !fs_ext::exists(path)? {
			return Ok(None);
		}
		let bytes = Bytes::from(fs_ext::read(path)?);
		let version = BlobVersion::of(&bytes);
		Some((bytes, version)).xok()
	}

	/// Run `write` on the file at `path` when `condition` holds of it, checked
	/// and written under its directory's locks.
	///
	/// One blocking step holds the process lock, so a caller dropped midway
	/// leaves a write made or not made, never one landing after the lock is
	/// free.
	async fn conditional<T: 'static + Send>(
		path: AbsPath,
		rel: FwdPath,
		condition: BlobCondition,
		write: impl 'static + Send + FnOnce(&AbsPath, bool) -> Result<T>,
	) -> Result<T> {
		let Some(dir) = path.parent() else {
			bevybail!("`{path}` is no file in a directory");
		};
		let _guard = Self::conditional_lock(&dir).lock().await;
		async_ext::unblock(move || {
			// a missing directory holds no file, and nobody to race for it
			// but a writer creating it, which locks it first
			if !matches!(condition, BlobCondition::Matches(_)) {
				fs_ext::create_dir_all(&dir)?;
			}
			let _lock = match fs_ext::exists(&dir)? {
				true => Some(fs_ext::lock_dir(&dir)?),
				false => None,
			};
			let found = Self::read_versioned(&path)?;
			BlobConflict::check(
				&condition,
				&rel,
				found.as_ref().map(|(_, version)| version),
			)?;
			write(&path, found.is_some())
		})
		.await
	}
}

impl StoreBackend for FsStore {
	fn id(&self) -> &'static str { "fs" }

	fn describe(&self) -> String { self.root_key().to_string() }

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
}

impl BlobProvider for FsStore {
	fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: FwdPath) -> Box<dyn BlobProvider> {
		Box::new(FsStore {
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
			..self.clone()
		})
	}

	fn base(&self) -> Box<dyn BlobProvider> {
		Box::new(FsStore {
			subdir: None,
			..self.clone()
		})
	}

	/// The filesystem has a parent universe above the store's root, so a
	/// `../..` root re-roots the store at the absolute resolved directory
	/// rather than erroring — walking above the entry's directory is the
	/// point of an fs `<PackageRoot>`.
	fn rebase(
		&self,
		entry_name: &FwdPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobProvider>, FwdPath)> {
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
			}) as Box<dyn BlobProvider>),
			entry_name,
		)
			.xok()
	}

	fn root_key(&self) -> SmolStr { format!("fs:{}", self.path).into() }

	fn subdir(&self) -> FwdPath { self.subdir.clone().unwrap_or_default() }

	fn watch_dir(&self) -> Option<AbsPath> { Some(self.effective_root()) }

	fn base_dir(&self) -> Option<AbsPath> { Some(self.path.clone()) }

	fn insert(&self, path: &FwdPath, body: Bytes) -> SendBoxedFuture<Result> {
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
		path: &FwdPath,
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

	fn list(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
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
					AbsPath::new_unchecked(path.to_string_lossy())
						.xmap(|path| FsStore::key_of(&root, &path))
				})
				.collect()
		})
	}

	/// One `read_dir`, a missing directory empty.
	fn list_dir(&self, path: &FwdPath) -> SendBoxedFuture<Result<BlobDir>> {
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

	fn get(&self, path: &FwdPath) -> SendBoxedFuture<Result<Bytes>> {
		let key = path.clone();
		let path = self.resolve_path(path);
		Box::pin(async move {
			// a miss is a 404 naming the blob, as every backend's is, so a
			// caller answering with it says what was missing; any other failure
			// is itself
			fs_ext::read_async(&path)
				.await
				.map_err(|err| -> BevyError {
					match err {
						FsError::FileNotFound { .. }
						| FsError::DirNotFound { .. } => HttpError::new(
							StatusCode::NOT_FOUND,
							format!("blob `{key}` not found"),
						)
						.into(),
						err => err.into(),
					}
				})?
				.xmap(Bytes::from)
				.xok()
		})
	}

	fn exists(&self, path: &FwdPath) -> SendBoxedFuture<Result<bool>> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::exists_async(path).await?.xok() })
	}

	fn remove(&self, path: &FwdPath) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::remove_async(path).await?.xok() })
	}

	fn conditional_writes(&self) -> bool { true }

	fn get_versioned(
		&self,
		path: &FwdPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			async_ext::unblock(move || Self::read_versioned(&path)).await
		})
	}

	/// Checked and written under the directory's locks, this process's and
	/// every other's ([`FsStore`]'s docs), the version the content's digest,
	/// the file replaced whole so an unlocked reader never reads it torn.
	fn insert_if(
		&self,
		path: &FwdPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		let rel = path.clone();
		let path = self.resolve_path(path);
		Box::pin(Self::conditional(
			path,
			rel,
			condition,
			move |path, _existed| {
				fs_ext::write_replacing(path, &body)?;
				BlobVersion::of(&body).xok()
			},
		))
	}

	fn remove_if(
		&self,
		path: &FwdPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		let rel = path.clone();
		let path = self.resolve_path(path);
		Box::pin(Self::conditional(path, rel, condition, |path, existed| {
			if existed {
				fs_ext::remove(path)?;
			}
			Ok(())
		}))
	}

	/// Digested in chunks on the blocking pool, so a stat of a multi-gigabyte
	/// file never holds it.
	fn stat(
		&self,
		path: &FwdPath,
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
	fn size(&self, path: &FwdPath) -> SendBoxedFuture<Result<Option<u64>>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			if !fs_ext::exists_async(&path).await? {
				return Ok(None);
			}
			fs_ext::file_size(&path)?.xsome().xok()
		})
	}

	/// Every followed link at any depth, a directory's own links included.
	fn empty_links(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
		let root = self.effective_root();
		let follow = self.follow_symlinks;
		Box::pin(async move {
			if !follow {
				return Vec::new().xok();
			}
			async_ext::unblock(move || {
				let mut found = Vec::new();
				let mut dirs = vec![root.clone()];
				while let Some(dir) = dirs.pop() {
					for link in fs_ext::symlinks(&dir)? {
						// a link to a file is no directory, and not empty
						if !fs_ext::exists(&link)?
							|| (fs_ext::is_dir(&link)
								&& fs_ext::is_dir_empty(&link)?)
						{
							let link =
								AbsPath::new_unchecked(link.to_string_lossy());
							found.push(FsStore::key_of(&root, &link)?);
						}
					}
					dirs.extend(ReadDir::dirs(&dir)?.into_iter().map(|dir| {
						AbsPath::new_unchecked(dir.to_string_lossy())
					}));
				}
				found.sort();
				found.xok()
			})
			.await
		})
	}

	fn public_url(
		&self,
		_path: &FwdPath,
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
			.with_subdir(fwd_path!("never/written"))
			.list()
			.await
			.unwrap()
			.xpect_eq(Vec::<FwdPath>::new());
	}

	/// The filesystem's parent universe lets a rebase walk above the store's
	/// current root: the store re-roots at the absolute resolved directory and
	/// the entry name grows the path back down to it.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn rebases_above_the_package_root() {
		let tmp = TempDir::new_ws().unwrap();
		let entry_dir = tmp.join("a/b");
		fs_ext::create_dir_all(&entry_dir).unwrap();
		fs_ext::write(entry_dir.join("main.bsx"), "<Router/>").unwrap();
		fs_ext::write(tmp.join("shared.txt"), "shared").unwrap();
		let store = BlobStore::new(FsStore::new(entry_dir));
		let (rebased, entry_name) =
			store.rebase_package("main.bsx", "../..").unwrap();
		entry_name.xpect_eq("a/b/main.bsx");
		rebased
			.get_media(&fwd_path!("a/b/main.bsx"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("<Router/>");
		rebased
			.get_media(&fwd_path!("shared.txt"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("shared");
	}

	/// Two processes rewriting one file never erase each other's changes: the
	/// other is this binary again, running [`second_process`].
	#[cfg(all(unix, feature = "json"))]
	#[beet_core::test(timeout_ms = 60_000)]
	async fn processes_share_a_file() {
		let dir = TempDir::new_ws().unwrap();
		let mut child = spawn_second_process(dir.path(), "counts");
		count(dir.path()).await;
		async_ext::unblock(move || child.wait().unwrap())
			.await
			.success()
			.xpect_true();
		JsonTableStore::over(BlobStore::new(FsStore::new(dir.path().clone())))
			.unwrap()
			.table::<table_test::NamedRow>()
			.get("n")
			.await
			.unwrap()
			.unwrap()
			.value
			.xpect_eq(COUNTS * 2);
	}

	/// A process holding the lock holds a write off, through a symlink to the
	/// directory too, and the moment it is killed the write goes through.
	#[cfg(unix)]
	#[beet_core::test(timeout_ms = 60_000)]
	async fn a_killed_process_leaves_no_lock() {
		let dir = TempDir::new_ws().unwrap();
		let links = TempDir::new_ws().unwrap();
		std::os::unix::fs::symlink(dir.path(), links.join("link")).unwrap();
		let mut child = spawn_second_process(dir.path(), "holds");
		while !fs_ext::exists(dir.join("held")).unwrap() {
			time_ext::sleep_millis(10).await;
		}
		let store = BlobStore::new(FsStore::new(links.join("link")));
		let written = store.clone();
		let write = async move {
			written
				.insert_if(
					&fwd_path!("a.txt"),
					"a".into(),
					BlobCondition::Absent,
				)
				.await
		};
		let kill = async {
			time_ext::sleep_millis(500).await;
			// still held off, then let go by the kill alone
			fs_ext::exists(dir.join("a.txt")).unwrap().xpect_false();
			child.kill().unwrap();
			child.wait().unwrap();
		};
		let (written, ()) =
			beet_core::exports::futures_lite::future::zip(write, kill).await;
		written.unwrap();
		store
			.exists(&fwd_path!("a.txt"))
			.await
			.unwrap()
			.xpect_true();
		// nothing to clean up
		let mut paths = store.list().await.unwrap();
		paths.sort();
		paths.xpect_eq(vec![fwd_path!("a.txt"), fwd_path!("held")]);
	}

	/// How many each process counts.
	#[cfg(all(unix, feature = "json"))]
	const COUNTS: i64 = 40;

	/// Count one row up, conditionally, yielding so writers interleave.
	#[cfg(all(unix, feature = "json"))]
	async fn count(dir: &AbsPath) {
		let rows =
			JsonTableStore::over(BlobStore::new(FsStore::new(dir.clone())))
				.unwrap()
				.table::<table_test::NamedRow>();
		for _ in 0..COUNTS {
			loop {
				let (row, precondition) =
					match rows.get_with_cid("n").await.unwrap() {
						Some((row, cid)) => (row, Precondition::Matches(cid)),
						None => (
							table_test::NamedRow::new("n", 0),
							Precondition::Absent,
						),
					};
				let next = table_test::NamedRow::new("n", row.value + 1);
				match rows.put_if(next, precondition).await {
					Ok(_) => break,
					Err(err) if TableError::is_conflict(&err) => {}
					Err(err) => panic!("{err}"),
				}
			}
		}
	}

	/// Where [`second_process`] is told its directory and its part.
	#[cfg(unix)]
	const SECOND_PROCESS: &str = "BEET_TEST_FS_STORE_SECOND_PROCESS";

	#[cfg(unix)]
	fn spawn_second_process(dir: &AbsPath, part: &str) -> std::process::Child {
		process_ext::this_test(["second_process", "--ignored"])
			.unwrap()
			.env(SECOND_PROCESS, format!("{part}:{dir}"))
			.stdout(std::process::Stdio::null())
			.spawn()
			.unwrap()
	}

	/// The other process of the tests above, run by them alone.
	#[cfg(unix)]
	#[beet_core::test(timeout_ms = 60_000)]
	#[ignore = "the second process of `processes_share_a_file` and `a_killed_process_leaves_no_lock`"]
	async fn second_process() {
		let Ok(told) = env_ext::var(SECOND_PROCESS) else {
			return;
		};
		let (part, dir) = told.split_once(':').unwrap();
		let dir = AbsPath::new(dir).unwrap();
		match part {
			#[cfg(feature = "json")]
			"counts" => count(&dir).await,
			_ => {
				let lock = fs_ext::lock_dir(&dir).unwrap();
				lock.is_held().xpect_true();
				fs_ext::write(dir.join("held"), "").unwrap();
				std::thread::sleep(Duration::from_secs(50));
				drop(lock);
			}
		}
	}
}
