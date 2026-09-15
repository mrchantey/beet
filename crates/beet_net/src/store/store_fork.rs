//! [`StoreFork`]: a local store forked off an upstream one, the fork a process
//! keeps of a repo it does not own.

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A local store forked off an upstream one: reads are local-first, falling
/// through to the upstream for a key the local does not hold; writes land
/// local; a listing is the union. The store-grain twin of
/// [`SceneFork`](beet_core::prelude::SceneFork), which forks one document
/// within a store.
///
/// The composition behind `--store-fork`: a browser forks the site's http repo
/// into IndexedDB, a terminal pulling a remote site forks it into an `fs` dir,
/// and a headless process on a remote repo composes the same pair and never
/// writes. It is a store composition with no surface dependency, so the first
/// edit to a published scene lands in the local store and every later boot
/// reads the fork from there while everything unedited still comes from
/// upstream. A key only the upstream holds cannot be removed: the fork keeps
/// no tombstones, so a removal reaches only the local copy.
///
/// Erased on construction rather than a reflected component: its two halves
/// are already erased stores, composed by whichever driver resolved them.
///
/// A change routes to the fork from three keys: its own base (what its
/// [`WatchDir`] keys the watcher's events to), the local half's and the
/// upstream's (a half spawned as its own store emits keyed to itself). Every
/// other store compares one key, so the fork answers
/// [`did_change`](BlobStoreProvider::did_change) and
/// [`matches_object`](BlobStoreProvider::matches_object) for all three.
#[derive(Clone)]
pub struct StoreFork {
	/// The store writes land in and reads try first.
	local: BlobStore,
	/// The store reads fall through to, never written.
	upstream: BlobStore,
}

impl StoreFork {
	/// `local` forked off `upstream`.
	pub fn new(local: BlobStore, upstream: BlobStore) -> Self {
		Self { local, upstream }
	}

	/// The local half, where writes land.
	pub fn local(&self) -> &BlobStore { &self.local }

	/// The upstream half, read through.
	pub fn upstream(&self) -> &BlobStore { &self.upstream }
}

impl BlobStoreProvider for StoreFork {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	/// Both halves scope together, so keys keep corresponding.
	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(Self {
			local: self.local.with_subdir(path.clone()),
			upstream: self.upstream.with_subdir(path),
		})
	}

	/// The pair of bases: what a watcher keys this store's events to.
	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(Self {
			local: self.local.base(),
			upstream: self.upstream.base(),
		})
	}

	/// Both halves rebase through their own rules, and must agree on where the
	/// entry landed: a local half that cannot follow the upstream above its
	/// root (a browser database under an `fs` upstream) is an error naming it,
	/// since keys that no longer correspond would silently shadow nothing.
	fn rebase(
		&self,
		entry_name: &RelPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobStoreProvider>, RelPath)> {
		let (upstream, upstream_entry) =
			self.upstream.rebase(entry_name, root)?;
		let (local, local_entry) = self.local.rebase(entry_name, root)?;
		if local_entry != upstream_entry {
			bevybail!(
				"store fork `{}` and its upstream `{}` disagree on the entry after \
				 rebasing to `{root}` (`{local_entry}` vs `{upstream_entry}`)",
				self.local.root_key(),
				self.upstream.root_key()
			);
		}
		(
			Box::new(Self {
				local: BlobStore::from_arc(local.into()),
				upstream: BlobStore::from_arc(upstream.into()),
			}) as Box<dyn BlobStoreProvider>,
			upstream_entry,
		)
			.xok()
	}

	fn id(&self) -> &'static str { "fork" }

	fn root_key(&self) -> SmolStr {
		format!(
			"fork:{}+{}",
			self.local.root_key(),
			self.upstream.root_key()
		)
		.into()
	}

	fn subdir(&self) -> RelPath { self.local.subdir() }

	/// The upstream's watch dir, else the local's: edits to either are edits
	/// to what this store reads.
	fn watch_dir(&self) -> Option<AbsPath> {
		self.upstream.watch_dir().or_else(|| self.local.watch_dir())
	}

	fn base_dir(&self) -> Option<AbsPath> {
		self.upstream.base_dir().or_else(|| self.local.base_dir())
	}

	/// A change keyed to this fork, or to either half, is a change to it.
	fn did_change(&self, event: &BlobEvent) -> bool {
		(self.root_key() == event.store.root_key()
			&& key_covers(&self.subdir(), &event.root_relative_path()))
			|| self.local.did_change(event)
			|| self.upstream.did_change(event)
	}

	/// The object at `path`, keyed to this fork or to either half.
	fn matches_object(&self, event: &BlobEvent, path: &RelPath) -> bool {
		(self.root_key() == event.store.root_key()
			&& self.subdir().join(path) == event.root_relative_path())
			|| self.local.matches_object(event, path)
			|| self.upstream.matches_object(event, path)
	}

	fn region(&self) -> Option<String> { self.upstream.region() }

	/// The upstream is the store; the local half is created on demand.
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.upstream.store_exists()
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.local.store_create()
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.local.store_remove()
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		BlobStoreProvider::insert(&self.local, path, body)
	}

	/// The union, each key once, in order.
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let local = self.local.list();
		let upstream = self.upstream.list();
		Box::pin(async move {
			let mut keys = local.await?;
			for key in upstream.await? {
				if !keys.contains(&key) {
					keys.push(key);
				}
			}
			keys.sort();
			keys.xok()
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let (local, upstream) = (self.local.clone(), self.upstream.clone());
		let path = path.clone();
		Box::pin(async move {
			match local.exists(&path).await? {
				true => local.get(&path).await,
				false => upstream.get(&path).await,
			}
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let (local, upstream) = (self.local.clone(), self.upstream.clone());
		let path = path.clone();
		Box::pin(async move {
			match local.exists(&path).await? {
				true => Ok(true),
				false => upstream.exists(&path).await,
			}
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.local.remove(path)
	}

	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let (local, upstream) = (self.local.clone(), self.upstream.clone());
		let path = path.clone();
		Box::pin(async move {
			match local.exists(&path).await? {
				true => local.public_url(&path).await,
				false => upstream.public_url(&path).await,
			}
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bytes::Bytes;

	/// A fork of two seeded memory stores: `shared.txt` in both (the local
	/// copy edited), `upstream.txt` only upstream, `local.txt` only local.
	async fn fork() -> (BlobStore, BlobStore, BlobStore) {
		let local = BlobStore::temp();
		let upstream = BlobStore::temp();
		for (store, path, body) in [
			(&upstream, "shared.txt", "published"),
			(&upstream, "upstream.txt", "upstream"),
			(&local, "shared.txt", "edited"),
			(&local, "local.txt", "local"),
		] {
			store.insert(&RelPath::new(path), body).await.unwrap();
		}
		let fork =
			BlobStore::new(StoreFork::new(local.clone(), upstream.clone()));
		(fork, local, upstream)
	}

	/// A read is local-first: a key both hold reads the local copy, a key
	/// only the upstream holds falls through, a key neither holds is a miss.
	#[beet_core::test]
	async fn reads_local_first() {
		let (fork, ..) = fork().await;
		let read = async |path: &str| {
			fork.get(&RelPath::new(path))
				.await
				.map(|bytes| String::from_utf8_lossy(&bytes).to_string())
		};
		read("shared.txt").await.unwrap().xpect_eq("edited");
		read("upstream.txt").await.unwrap().xpect_eq("upstream");
		read("local.txt").await.unwrap().xpect_eq("local");
		read("missing.txt").await.xpect_err();
		fork.exists(&RelPath::new("upstream.txt"))
			.await
			.unwrap()
			.xpect_true();
		fork.exists(&RelPath::new("missing.txt"))
			.await
			.unwrap()
			.xpect_false();
	}

	/// A write lands local and never reaches the upstream, and a removal
	/// reaches only the local copy so the upstream's shows through again.
	#[beet_core::test]
	async fn writes_land_local() {
		let (fork, local, upstream) = fork().await;
		fork.insert(&RelPath::new("new.txt"), Bytes::from_static(b"new"))
			.await
			.unwrap();
		local
			.exists(&RelPath::new("new.txt"))
			.await
			.unwrap()
			.xpect_true();
		upstream
			.exists(&RelPath::new("new.txt"))
			.await
			.unwrap()
			.xpect_false();
		fork.remove(&RelPath::new("shared.txt")).await.unwrap();
		fork.get(&RelPath::new("shared.txt"))
			.await
			.unwrap()
			.xpect_eq(Bytes::from_static(b"published"));
	}

	/// A listing is the union, each key once, and a subdir scopes both halves.
	#[beet_core::test]
	async fn lists_the_union() {
		let (fork, ..) = fork().await;
		fork.list().await.unwrap().xpect_eq(vec![
			RelPath::new("local.txt"),
			RelPath::new("shared.txt"),
			RelPath::new("upstream.txt"),
		]);
		let scoped = fork.with_subdir(RelPath::new("docs"));
		scoped
			.insert(&RelPath::new("a.md"), Bytes::from_static(b"a"))
			.await
			.unwrap();
		fork.exists(&RelPath::new("docs/a.md"))
			.await
			.unwrap()
			.xpect_true();
		scoped
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("a.md")]);
	}

	/// Both halves rebase together through an entry's `<RepoRoot>`, so a key
	/// keeps naming the same file on each side.
	/// An event keyed to either half or to the fork's own base (what its
	/// watcher keys to) is a change to a scoped fork and to its blob, so a
	/// fork-backed repo store reloads and its documents re-read; a sibling
	/// object or an unrelated store is neither.
	#[beet_core::test]
	fn routes_events_from_either_half_and_its_base() {
		let (local, upstream) = (BlobStore::temp(), BlobStore::temp());
		let fork =
			BlobStore::new(StoreFork::new(local.clone(), upstream.clone()));
		let scoped = fork.with_subdir(RelPath::from("docs"));
		let blob = scoped.blob(RelPath::from("a.md"));
		let event = |store: &BlobStore, path: &str| {
			BlobEvent::new(
				store.clone(),
				RelPath::from(path),
				BlobEventKind::Changed,
			)
		};
		for source in [&local, &upstream, &fork.base()] {
			scoped.did_change(&event(source, "docs/a.md")).xpect_true();
			blob.matches_event(&event(source, "docs/a.md")).xpect_true();
			blob.matches_event(&event(source, "docs/b.md"))
				.xpect_false();
		}
		let unrelated = BlobStore::temp();
		scoped
			.did_change(&event(&unrelated, "docs/a.md"))
			.xpect_false();
		blob.matches_event(&event(&unrelated, "docs/a.md"))
			.xpect_false();
	}

	#[beet_core::test]
	async fn rebases_both_halves() {
		let local = BlobStore::temp();
		let upstream = BlobStore::temp();
		upstream
			.insert(&RelPath::new("site/app/main.bsx"), "<Router/>")
			.await
			.unwrap();
		let fork = BlobStore::new(StoreFork::new(local, upstream));
		let (rebased, entry_name) =
			fork.rebase_repo("site/app/main.bsx", "..").unwrap();
		entry_name.xpect_eq("app/main.bsx");
		rebased
			.exists(&RelPath::new("app/main.bsx"))
			.await
			.unwrap()
			.xpect_true();
		rebased
			.insert(&RelPath::new("app/fork.json"), Bytes::from_static(b"{}"))
			.await
			.unwrap();
		fork.exists(&RelPath::new("site/app/fork.json"))
			.await
			.unwrap()
			.xpect_true();
	}
}
