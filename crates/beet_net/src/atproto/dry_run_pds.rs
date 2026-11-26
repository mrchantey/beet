//! A repo a dry run writes to, leaving the repo beneath it untouched.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bytes::Bytes;

/// A [`PdsProvider`] over another repo that keeps every write in memory:
/// reads fall through to the repo beneath unless this run wrote or deleted
/// the record, a blob uploads nowhere, its reference computed locally. So a
/// dry run is the real run against a dry repo, and each of its steps reads
/// what the steps before it would have written.
///
/// [`Pds::dry`] makes one, and a [`ConvergeReport`] over one words its lines
/// as what a run would do.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async_ext::block_on(async {
/// let pds = Pds::temp();
/// let dry = pds.dry();
/// let collection = Nsid::new_static("com.example.note");
/// let rkey = Rkey::parse("first").unwrap();
/// dry.put_record(&collection, Rkeyed::new(rkey.clone(), value!({ "text": "hi" }).try_into()?))
/// 	.await?;
/// dry.get_record(&collection, &rkey).await?.xpect_some();
/// pds.get_record(&collection, &rkey).await?.xpect_none();
/// # Ok::<_, BevyError>(())
/// # })
/// # .unwrap();
/// ```
#[derive(Clone)]
pub struct DryRunPds {
	/// The repo beneath, only ever read.
	repo: Pds,
	/// What this run wrote (`Some`) or deleted (`None`), by scope, collection
	/// and rkey, shared by every scope of the run.
	writes: Arc<RwLock<HashMap<(RepoScope, Nsid, Rkey), Option<RecordEntry>>>>,
}

impl DryRunPds {
	/// Provider id, see [`PdsProvider::id`].
	pub const ID: &'static str = "dry-run";

	/// A dry repo over `repo`, nothing written yet.
	pub fn new(repo: Pds) -> Self {
		Self {
			repo,
			writes: default(),
		}
	}

	/// What this run did at `collection` and `rkey`: `None` when it did
	/// nothing there, `Some(None)` when it deleted the record.
	fn written(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
	) -> Option<Option<RecordEntry>> {
		self.writes
			.read()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner)
			.get(&(self.repo.scope().clone(), collection.clone(), rkey.clone()))
			.cloned()
	}

	/// Record `entry` (`None` deletes) as this run's at `collection` and
	/// `rkey`.
	fn write(&self, collection: Nsid, rkey: Rkey, entry: Option<RecordEntry>) {
		self.writes
			.write()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner)
			.insert((self.repo.scope().clone(), collection, rkey), entry);
	}

	async fn get(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> Result<Option<RecordEntry>> {
		match self.written(&collection, &rkey) {
			Some(entry) => entry.xok(),
			None => self.repo.get_record(&collection, &rkey).await,
		}
	}

	async fn list(&self, collection: Nsid) -> Result<Vec<RecordEntry>> {
		let mut entries = self
			.repo
			.list_records(&collection)
			.await?
			.into_iter()
			.map(|entry| (entry.rkey().clone(), entry))
			.collect::<HashMap<_, _>>();
		let writes = self
			.writes
			.read()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner)
			.iter()
			.filter(|((scope, written, _), _)| {
				scope == self.repo.scope() && *written == collection
			})
			.map(|((_, _, rkey), entry)| (rkey.clone(), entry.clone()))
			.collect::<Vec<_>>();
		for (rkey, entry) in writes {
			match entry {
				Some(entry) => entries.insert(rkey, entry),
				None => entries.remove(&rkey),
			};
		}
		let mut entries = entries.into_values().collect::<Vec<_>>();
		entries.sort_by(|left, right| left.rkey().cmp(right.rkey()));
		entries.xok()
	}
}

impl PdsProvider for DryRunPds {
	fn box_clone(&self) -> Box<dyn PdsProvider> { Box::new(self.clone()) }

	fn id(&self) -> &'static str { Self::ID }

	fn describe(&self) -> String {
		format!("{} over {}", Self::ID, self.repo.describe())
	}

	fn did(&self) -> &Did { self.repo.did() }

	fn is_dry_run(&self) -> bool { true }

	fn scope(&self) -> &RepoScope { self.repo.scope() }

	fn scoped(&self, scope: RepoScope) -> Result<Box<dyn PdsProvider>> {
		let scoped: Box<dyn PdsProvider> = Box::new(Self {
			repo: self.repo.scoped(scope)?,
			writes: self.writes.clone(),
		});
		scoped.xok()
	}

	fn get_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result<Option<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.get(collection, rkey).await })
	}

	fn put_record(
		&self,
		collection: Nsid,
		record: Rkeyed<AtprotoValue>,
	) -> SendBoxedFuture<Result<StrongRef>> {
		let this = self.clone();
		Box::pin(async move {
			let rkey = record.rkey().clone();
			let entry = RecordEntry {
				uri: this.repo.uri(&collection, &rkey),
				cid: record.cid(),
				value: record.into_value(),
			};
			let strong_ref = entry.strong_ref();
			this.write(collection, rkey, Some(entry));
			strong_ref.xok()
		})
	}

	fn delete_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result> {
		self.write(collection, rkey, None);
		Box::pin(async { Ok(()) })
	}

	fn list_records(
		&self,
		collection: Nsid,
	) -> SendBoxedFuture<Result<Vec<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.list(collection).await })
	}

	fn upload_blob(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> SendBoxedFuture<Result<BlobRef>> {
		let blob = BlobRef::of(&bytes, mime_type);
		Box::pin(async move { blob.xok() })
	}
}

/// The dry repo of each account one run writes to, carried in its request's
/// [`Extensions`] so every step a sequence threads the request through
/// writes to, and reads from, the same dry repos.
#[derive(Default)]
pub struct DryRunRepos(HashMap<Did, Pds>);

impl DryRunRepos {
	/// `pds` as the run `request` is sees it: unchanged unless `dry_run`,
	/// else the run's dry repo over it, made the first time a step asks.
	pub fn resolve(request: &mut Request, pds: Pds, dry_run: bool) -> Pds {
		if !dry_run {
			return pds;
		}
		request
			.extensions
			.get_or_default::<Self>()
			.0
			.entry(pds.did().clone())
			.or_insert_with(|| pds.dry())
			.clone()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A note at `rkey`.
	fn note(rkey: &str, text: &str) -> Rkeyed<AtprotoValue> {
		Rkeyed::new(
			Rkey::parse(rkey).unwrap(),
			value!({ "text": text }).try_into().unwrap(),
		)
	}

	const NOTES: Nsid = Nsid::new_static("com.example.note");

	/// A listing reads the repo beneath with this run's writes and deletes
	/// laid over it, in rkey order, and the repo beneath is untouched.
	#[beet_core::test]
	async fn lists_its_writes_over_the_repo() {
		let pds = Pds::temp();
		let written =
			pds.put_record(&NOTES, note("bbb", "kept")).await.unwrap();
		pds.put_record(&NOTES, note("ddd", "gone")).await.unwrap();
		let dry = pds.dry();
		dry.is_dry_run().xpect_true();
		let created = dry.put_record(&NOTES, note("aaa", "new")).await.unwrap();
		dry.delete_record(&NOTES, &Rkey::parse("ddd").unwrap())
			.await
			.unwrap();
		dry.list_records(&NOTES)
			.await
			.unwrap()
			.into_iter()
			.map(|entry| entry.strong_ref())
			.collect::<Vec<_>>()
			.xpect_eq(vec![created.clone(), written]);
		// the cid is the one the repo answers for the same write
		pds.put_record(&NOTES, note("aaa", "new"))
			.await
			.unwrap()
			.xpect_eq(created);
		pds.list_records(&NOTES).await.unwrap().len().xpect_eq(3);
	}

	/// One run's steps share one dry repo per account through the request.
	#[beet_core::test]
	async fn a_request_carries_its_dry_repos() {
		let pds = Pds::temp();
		let mut request = Request::get("/publish");
		DryRunRepos::resolve(&mut request, pds.clone(), false)
			.is_dry_run()
			.xpect_false();
		DryRunRepos::resolve(&mut request, pds.clone(), true)
			.put_record(&NOTES, note("aaa", "new"))
			.await
			.unwrap();
		DryRunRepos::resolve(&mut request, pds.clone(), true)
			.get_record(&NOTES, &Rkey::parse("aaa").unwrap())
			.await
			.unwrap()
			.xpect_some();
		pds.list_records(&NOTES).await.unwrap().xpect_empty();
	}
}
