//! An atproto repo laid out in any [`BlobStore`], doing a PDS's duties on write.
use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A [`PdsProvider`] over any [`BlobStore`]: the PDS a test writes to, and
/// the one a package's space that never leaves the machine (or lives in a
/// bucket) is.
///
/// Authoritative on the layout, which is plain enough to read in a file
/// browser and to mirror with `BlobSync`. Each [`RepoScope`] is a
/// self-contained root with its own records and blobs, the public repo at the
/// top and each member's repo within a space beneath it, the account's own
/// under `self`, so one root is one partition and a blob of a space never sits
/// in the pool a public bucket serves:
///
/// ```text
/// records/<collection>/<rkey>.cbor   a record body in its storage format
/// blobs/<cid>                        a blob's bytes
/// space/<type>/<key>/<author>/       a member's repo in a space, laid out
///                                    the same, `space` the uri's segment
/// ```
///
/// A record is stored in its [`storage_format`](Self::storage_format),
/// [`MediaType::DagCbor`] by default, through [`MediaType::serialize`] and
/// [`MediaType::deserialize`], and named with that format's extension so a
/// store that picks a media type by extension serves it correctly; an rkey may
/// hold a dot, so exactly one suffix is stripped. Under DAG-CBOR a record file
/// is exactly the block its cid hashes, so a read re-encodes nothing;
/// [`with_storage_format`](Self::with_storage_format) selects
/// [`MediaType::Json`] for a bucket meant to be read by eye.
///
/// And authoritative on the PDS's duties on write, so a repo here and one on a
/// real PDS cannot drift apart:
///
/// - the record cid is computed over canonical DAG-CBOR under CIDv1, exactly
///   as a PDS computes it ([`AtprotoValue::cid`]), so a record's strong ref
///   and any rkey derived from it are the same in a bucket and on the network;
/// - a record's `$type` must be its collection, and every blob it references
///   must have been uploaded;
/// - a blob is retained exactly while a current record references it: a write
///   or delete that drops a record's last reference to a blob deletes the
///   blob, and [`sweep`](Self::sweep) collects uploads no record ever took up,
///   the PDS's periodic garbage collection;
/// - a listing is per collection, in rkey order.
#[derive(Debug, Clone, Get, SetWith)]
pub struct EmulatorPds {
	#[get(skip)]
	#[set_with(skip)]
	did: Did,
	/// The account's whole tree, every scope beneath it.
	#[set_with(skip)]
	root: BlobStore,
	#[get(skip)]
	#[set_with(skip)]
	scope: RepoScope,
	/// The store this scope's repo is laid out in.
	#[set_with(skip)]
	store: BlobStore,
	/// The format a record body is stored in, [`MediaType::DagCbor`] by
	/// default.
	storage_format: MediaType,
}

impl EmulatorPds {
	/// The provider id.
	pub const ID: &'static str = "emulator";
	/// The did a test repo answers for when none is given.
	pub const TEST_DID: &'static str = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";

	const RECORDS: FwdPath = fwd_path!("records");
	const BLOBS: FwdPath = fwd_path!("blobs");
	/// The directory of every space, named as its at uri segment.
	const SPACES: FwdPath = fwd_path!(RepoScope::SPACE_SEGMENT);
	/// The author directory of the account's own repo in a space.
	const SELF_AUTHOR: PathSegment = path_segment!("self");

	/// The public repo of `did` laid out in `store`, its records in DAG-CBOR.
	pub fn new(did: Did, store: BlobStore) -> Self {
		Self {
			did,
			root: store.clone(),
			scope: RepoScope::Public,
			store,
			storage_format: MediaType::DagCbor,
		}
	}

	/// This account's repo `scope`, laid out beneath the same root.
	pub fn with_scope(&self, scope: RepoScope) -> Self {
		let store = match &scope {
			RepoScope::Public => self.root.clone(),
			RepoScope::Space { space, author } => {
				self.root.with_subdir(
					Self::SPACES
						.join_segment(&PathSegment::of(&space.r#type))
						.join_segment(&PathSegment::of(&space.key))
						.join_segment(&match author == &self.did {
							true => Self::SELF_AUTHOR,
							false => PathSegment::of(author),
						}),
				)
			}
		};
		Self {
			scope,
			store,
			..self.clone()
		}
	}

	/// A repo in a fresh in-memory store, answering for [`Self::TEST_DID`].
	pub fn temp() -> Self {
		Self::new(
			Did::parse(Self::TEST_DID).expect("the test did is a did:plc"),
			BlobStore::temp(),
		)
	}

	/// Delete every blob no current record references, answering their cids:
	/// what a PDS does to an upload no record took up.
	pub async fn sweep(&self) -> Result<Vec<Cid>> {
		let blobs = self
			.store
			.list_dir(&Self::BLOBS)
			.await?
			.files
			.into_iter()
			.map(|file| Cid::parse(&file))
			.collect::<Result<HashSet<_>>>()?;
		self.release(blobs).await
	}

	/// The suffix a record file carries, `.` and the storage format's
	/// extension.
	fn suffix(&self) -> String {
		format!(".{}", self.storage_format.extension().unwrap_or("bin"))
	}

	fn record_path(&self, collection: &Nsid, rkey: &Rkey) -> Result<FwdPath> {
		Self::RECORDS.try_join(format!("{collection}/{rkey}{}", self.suffix()))
	}

	fn blob_path(cid: &Cid) -> FwdPath { Self::BLOBS.join(&cid.into()) }

	/// The body stored at `path`, `None` when there is none.
	async fn read(&self, path: &FwdPath) -> Result<Option<AtprotoValue>> {
		if !self.store.exists(path).await? {
			return Ok(None);
		}
		let bytes = self.store.get(path).await?;
		self.storage_format
			.deserialize::<AtprotoValue>(&bytes)?
			.xmap(Some)
			.xok()
	}

	async fn get(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
	) -> Result<Option<RecordEntry>> {
		self.read(&self.record_path(collection, rkey)?)
			.await?
			.map(|value| {
				self.entry(collection, Rkeyed::new(rkey.clone(), value))
			})
			.xok()
	}

	fn uri(&self, collection: &Nsid, rkey: &Rkey) -> AtUri {
		AtUri::scoped(
			self.did.clone(),
			self.scope.clone(),
			collection.clone(),
			rkey.clone(),
		)
	}

	fn entry(
		&self,
		collection: &Nsid,
		record: Rkeyed<AtprotoValue>,
	) -> RecordEntry {
		RecordEntry {
			uri: self.uri(collection, record.rkey()),
			cid: record.cid(),
			value: record.into_value(),
		}
	}

	async fn put(
		&self,
		collection: Nsid,
		record: Rkeyed<AtprotoValue>,
		precondition: Precondition,
	) -> Result<StrongRef> {
		let record = record.try_map(|body| body.into_record(&collection))?;
		let uri = self.uri(&collection, record.rkey());
		let referenced = Self::blob_cids(&record);
		for blob in &referenced {
			if !self.store.exists(&Self::blob_path(blob)).await? {
				bevybail!(
					"{uri} references blob {blob}, which was never uploaded \
					 to this repo"
				);
			}
		}
		let path = self.record_path(&collection, record.rkey())?;
		let condition = self.precondition(&uri, &path, &precondition).await?;
		let dropped = self.dropped_blobs(&path, &referenced).await?;
		let bytes = self.storage_format.serialize(&*record)?;
		match condition {
			BlobCondition::Any => self.store.insert(&path, bytes).await?,
			condition => {
				if let Err(err) =
					self.store.insert_if(&path, bytes.into(), condition).await
				{
					// the record moved between the check and the write
					if err.downcast_ref::<BlobConflict>().is_none() {
						return Err(err);
					}
					return Err(RecordConflict {
						found: self.read(&path).await?.map(|value| value.cid()),
						uri,
						expected: precondition,
					}
					.into());
				}
			}
		}
		self.release(dropped).await?;
		StrongRef::new(uri, record.cid()).xok()
	}

	/// Check `precondition` against the record at `path`, answering the
	/// condition the write then makes on the stored object, so a store with
	/// conditional writes checks and writes as one step. A store without them
	/// checks before the write, which holds for one writer.
	async fn precondition(
		&self,
		uri: &AtUri,
		path: &FwdPath,
		precondition: &Precondition,
	) -> Result<BlobCondition> {
		if *precondition == Precondition::Any {
			return BlobCondition::Any.xok();
		}
		let decode = |bytes: &Bytes| -> Result<Cid> {
			self.storage_format
				.deserialize::<AtprotoValue>(bytes)?
				.cid()
				.xok()
		};
		if !self.store.conditional_writes() {
			let found = self.read(path).await?.map(|value| value.cid());
			RecordConflict::check(precondition, uri, found.as_ref())?;
			return BlobCondition::Any.xok();
		}
		match self.store.get_versioned(path).await? {
			Some((bytes, version)) => {
				RecordConflict::check(
					precondition,
					uri,
					Some(&decode(&bytes)?),
				)?;
				BlobCondition::Matches(version)
			}
			None => {
				RecordConflict::check(precondition, uri, None)?;
				BlobCondition::Absent
			}
		}
		.xok()
	}

	async fn delete(&self, collection: Nsid, rkey: Rkey) -> Result {
		let path = self.record_path(&collection, &rkey)?;
		let dropped = self.dropped_blobs(&path, &HashSet::default()).await?;
		if self.store.exists(&path).await? {
			self.store.remove(&path).await?;
		}
		self.release(dropped).await?;
		Ok(())
	}

	async fn list(&self, collection: &Nsid) -> Result<Vec<RecordEntry>> {
		let suffix = self.suffix();
		let mut rkeys = self
			.store
			.list_dir(&Self::RECORDS.try_join(collection.to_string())?)
			.await?
			.files
			.into_iter()
			.filter_map(|file| {
				file.strip_suffix(suffix.as_str()).map(Rkey::parse)
			})
			.collect::<Result<Vec<_>>>()?;
		rkeys.sort();
		let mut entries = Vec::with_capacity(rkeys.len());
		for rkey in rkeys {
			if let Some(entry) = self.get(collection, &rkey).await? {
				entries.push(entry);
			}
		}
		entries.xok()
	}

	async fn upload(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> Result<BlobRef> {
		let blob = BlobRef::of(&bytes, mime_type);
		let path = Self::blob_path(&blob.cid);
		if !self.store.exists(&path).await? {
			self.store.insert(&path, bytes).await?;
		}
		blob.xok()
	}

	/// The blobs the record at `path` references that `kept` does not.
	async fn dropped_blobs(
		&self,
		path: &FwdPath,
		kept: &HashSet<Cid>,
	) -> Result<HashSet<Cid>> {
		self.read(path)
			.await?
			.map(|old| Self::blob_cids(&old))
			.unwrap_or_default()
			.into_iter()
			.filter(|cid| !kept.contains(cid))
			.collect::<HashSet<_>>()
			.xok()
	}

	/// Delete each of `candidates` no current record references. Scans every
	/// record, so it runs only when a write actually dropped a reference.
	async fn release(&self, mut candidates: HashSet<Cid>) -> Result<Vec<Cid>> {
		if candidates.is_empty() {
			return Ok(Vec::new());
		}
		// this scope's records alone: a space beneath the public root is
		// another partition
		let records = self
			.store
			.list()
			.await?
			.into_iter()
			.filter(|path| path.strip_prefix(&Self::RECORDS).is_some());
		for path in records {
			if let Some(record) = self.read(&path).await? {
				for cid in Self::blob_cids(&record) {
					candidates.remove(&cid);
				}
			}
		}
		let mut released = candidates.into_iter().collect::<Vec<_>>();
		released.sort();
		for cid in &released {
			self.store.remove(&Self::blob_path(cid)).await?;
		}
		released.xok()
	}

	/// The cid of every blob `value` references.
	fn blob_cids(value: &AtprotoValue) -> HashSet<Cid> {
		value.blob_refs().into_iter().map(|blob| blob.cid).collect()
	}
}

impl PdsProvider for EmulatorPds {
	fn box_clone(&self) -> Box<dyn PdsProvider> { Box::new(self.clone()) }

	fn id(&self) -> &'static str { Self::ID }

	fn describe(&self) -> String {
		format!("{} {} in {}", Self::ID, self.did, self.store.describe())
	}

	fn did(&self) -> &Did { &self.did }

	fn scope(&self) -> &RepoScope { &self.scope }

	fn scoped(&self, scope: RepoScope) -> Result<Box<dyn PdsProvider>> {
		let scoped: Box<dyn PdsProvider> = Box::new(self.with_scope(scope));
		scoped.xok()
	}

	fn get_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result<Option<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.get(&collection, &rkey).await })
	}

	fn put_record(
		&self,
		collection: Nsid,
		record: Rkeyed<AtprotoValue>,
		precondition: Precondition,
	) -> SendBoxedFuture<Result<StrongRef>> {
		let this = self.clone();
		Box::pin(
			async move { this.put(collection, record, precondition).await },
		)
	}

	fn delete_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move { this.delete(collection, rkey).await })
	}

	fn list_records(
		&self,
		collection: Nsid,
	) -> SendBoxedFuture<Result<Vec<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.list(&collection).await })
	}

	fn upload_blob(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> SendBoxedFuture<Result<BlobRef>> {
		let this = self.clone();
		Box::pin(async move { this.upload(bytes, mime_type).await })
	}

	fn get_blob(&self, cid: Cid) -> SendBoxedFuture<Result<Option<Bytes>>> {
		let store = self.store.clone();
		Box::pin(async move {
			let path = Self::blob_path(&cid);
			match store.exists(&path).await? {
				true => store.get(&path).await?.xmap(Some).xok(),
				false => Ok(None),
			}
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn collection() -> Nsid { nsid!("com.example.card") }
	fn rkey(key: &str) -> Rkey { Rkey::parse(key).unwrap() }

	/// A fresh repo in each storage format this build can write: DAG-CBOR,
	/// and json under `json`.
	fn emulators() -> Vec<EmulatorPds> {
		#[allow(unused_mut)]
		let mut emulators = vec![EmulatorPds::temp()];
		#[cfg(feature = "json")]
		emulators
			.push(EmulatorPds::temp().with_storage_format(MediaType::Json));
		emulators
	}

	/// `value` at `key`.
	fn record(key: &str, value: Value) -> Rkeyed<AtprotoValue> {
		Rkeyed::new(rkey(key), value.try_into().unwrap())
	}

	/// A card at `key` holding `blob`, the shape every retention case
	/// writes.
	fn card(key: &str, blob: &BlobRef) -> Rkeyed<AtprotoValue> {
		record(key, value!({ "image": (Value::from_serde(blob).unwrap()) }))
	}

	/// The layout is the documented one, the record file named for its
	/// format, and the cid a written record answers is the one a read
	/// recomputes.
	#[beet_core::test]
	async fn lays_out_records_and_blobs() {
		for emulator in emulators() {
			let pds = Pds::new(emulator.clone());
			let blob = pds.upload_blob("png", MediaType::Png).await.unwrap();
			let written = pds
				.put_record(&collection(), card("a", &blob))
				.await
				.unwrap();
			let mut paths = emulator.store().list().await.unwrap();
			paths.sort();
			paths.xpect_eq(vec![
				FwdPath::new(format!("blobs/{}", blob.cid)).unwrap(),
				FwdPath::new(format!(
					"records/com.example.card/a.{}",
					emulator.storage_format().extension().unwrap()
				)).unwrap(),
			]);
			pds.get_record(&collection(), &rkey("a"))
				.await
				.unwrap()
				.unwrap()
				.strong_ref()
				.xpect_eq(written);
			pds.get_record(&collection(), &rkey("b"))
				.await
				.unwrap()
				.xpect_none();
		}
	}

	/// A DAG-CBOR record file is exactly the block its cid hashes, and a
	/// float survives both formats.
	#[beet_core::test]
	async fn stores_the_hashed_block() {
		let emulator = EmulatorPds::temp();
		let written = Pds::new(emulator.clone())
			.put_record(&collection(), record("a", value!({ "scale": 0.5 })))
			.await
			.unwrap();
		let file = emulator
			.store()
			.get(&fwd_path!("records/com.example.card/a.cbor"))
			.await
			.unwrap();
		Cid::new(Cid::DAG_CBOR, &file).xpect_eq(written.cid);
		for emulator in emulators() {
			let pds = Pds::new(emulator);
			pds.put_record(
				&collection(),
				record("a", value!({ "scale": 0.5 })),
			)
			.await
			.unwrap();
			pds.get_record(&collection(), &rkey("a"))
				.await
				.unwrap()
				.unwrap()
				.value
				.decode()
				.xpect_eq(
					value!({ "$type": "com.example.card", "scale": 0.5 }),
				);
		}
	}

	/// A json store in a build without `json` fails exactly as any json write
	/// and read does, so a disabled format fails identically everywhere.
	#[cfg(not(feature = "json"))]
	#[beet_core::test]
	async fn json_store_needs_json() {
		let message =
			|result: Result<()>| result.map_err(|err| err.to_string());
		let emulator = EmulatorPds::temp().with_storage_format(MediaType::Json);
		let pds = Pds::new(emulator.clone());
		pds.put_record(&collection(), record("a", value!({})))
			.await
			.map(|_| ())
			.xmap(message)
			.xpect_eq(message(
				MediaType::Json.serialize(&value!({})).map(|_| ()),
			));
		emulator
			.store()
			.insert(&fwd_path!("records/com.example.card/a.json"), "{}")
			.await
			.unwrap();
		pds.get_record(&collection(), &rkey("a"))
			.await
			.map(|_| ())
			.xmap(message)
			.xpect_eq(message(
				MediaType::Json
					.deserialize::<AtprotoValue>(b"{}")
					.map(|_| ()),
			));
	}

	/// A record's `$type` is its collection, and a blob it names must exist.
	#[beet_core::test]
	async fn refuses_what_a_pds_refuses() {
		for emulator in emulators() {
			let pds = Pds::new(emulator);
			pds.put_record(
				&collection(),
				record("a", value!({ "$type": "com.example.other" })),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("declares `$type");
			let blob = BlobRef::of(b"never uploaded", MediaType::Png);
			pds.put_record(&collection(), card("a", &blob))
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("never uploaded");
		}
	}

	/// A blob lives exactly while a current record references it.
	#[beet_core::test]
	async fn retains_referenced_blobs() {
		for emulator in emulators() {
			let pds = Pds::new(emulator.clone());
			let exists = async |blob: &BlobRef| {
				emulator
					.store()
					.exists(&FwdPath::new(format!("blobs/{}", blob.cid)).unwrap())
					.await
					.unwrap()
			};
			let first = pds.upload_blob("one", MediaType::Png).await.unwrap();
			let second = pds.upload_blob("two", MediaType::Png).await.unwrap();
			// two records share the first blob
			pds.put_record(&collection(), card("a", &first))
				.await
				.unwrap();
			pds.put_record(&collection(), card("b", &first))
				.await
				.unwrap();
			// replacing one keeps the blob the other still holds
			pds.put_record(&collection(), card("a", &second))
				.await
				.unwrap();
			exists(&first).await.xpect_true();
			// deleting the last holder releases it
			pds.delete_record(&collection(), &rkey("b")).await.unwrap();
			exists(&first).await.xpect_false();
			exists(&second).await.xpect_true();
			// an upload no record takes up is swept
			let orphan =
				pds.upload_blob("three", MediaType::Png).await.unwrap();
			emulator
				.sweep()
				.await
				.unwrap()
				.xpect_eq(vec![orphan.cid.clone()]);
			exists(&orphan).await.xpect_false();
			exists(&second).await.xpect_true();
		}
	}

	/// Each scope is a root of its own: a space's records and blobs sit
	/// beneath the public root, never in its pool, and its addresses carry
	/// the space.
	#[beet_core::test]
	async fn scopes_are_self_contained_roots() {
		let emulator = EmulatorPds::temp();
		let public = Pds::new(emulator.clone());
		let space = public.space(SpaceRef::package("todos").unwrap()).unwrap();
		let blob = space.upload_blob("private", MediaType::Png).await.unwrap();
		let written = space
			.put_record(&collection(), card("a", &blob))
			.await
			.unwrap();
		written.uri.scope().xpect_eq(space.scope().clone());
		written
			.uri
			.to_string()
			.xpect_contains("/space/org.beet.space.package/todos/did:plc:");
		let mut paths = emulator.store().list().await.unwrap();
		paths.sort();
		paths.xpect_eq(vec![
			FwdPath::new(format!(
				"space/org.beet.space.package/todos/self/blobs/{}",
				blob.cid
			)).unwrap(),
			FwdPath::new(
				"space/org.beet.space.package/todos/self/records/com.example.card/a.cbor",
			).unwrap(),
		]);
		public
			.get_record(&collection(), &rkey("a"))
			.await
			.unwrap()
			.xpect_none();
		// a public write retains nothing of the space and releases nothing
		// of it either
		let public_blob =
			public.upload_blob("public", MediaType::Png).await.unwrap();
		public
			.put_record(&collection(), card("a", &public_blob))
			.await
			.unwrap();
		public
			.delete_record(&collection(), &rkey("a"))
			.await
			.unwrap();
		emulator
			.store()
			.exists(&FwdPath::new(format!(
				"space/org.beet.space.package/todos/self/blobs/{}",
				blob.cid
			)).unwrap())
			.await
			.unwrap()
			.xpect_true();
		// another member's repo in the same space is its own root
		let other = Did::parse("did:plc:bbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
		public
			.scoped(RepoScope::space(
				SpaceRef::package("todos").unwrap(),
				other.clone(),
			))
			.unwrap()
			.get_record(&collection(), &rkey("a"))
			.await
			.unwrap()
			.xpect_none();
	}

	#[beet_core::test]
	async fn lists_a_collection_in_rkey_order() {
		for emulator in emulators() {
			let pds = Pds::new(emulator);
			for key in ["c", "a", "b"] {
				pds.put_record(
					&collection(),
					record(key, value!({ "key": key })),
				)
				.await
				.unwrap();
			}
			pds.put_record(
				&nsid!("com.example.other"),
				record("z", value!({})),
			)
			.await
			.unwrap();
			pds.list_records(&collection())
				.await
				.unwrap()
				.iter()
				.map(|entry| entry.rkey().to_string())
				.collect::<Vec<_>>()
				.xpect_eq(vec!["a", "b", "c"]);
			pds.delete_record(&collection(), &rkey("missing"))
				.await
				.unwrap();
		}
	}

	/// A create refuses a taken rkey and a swap a moved record, naming both
	/// cids, here and through a dry run over the repo alike.
	#[beet_core::test]
	async fn writes_under_a_precondition() {
		let repos = emulators().into_iter().map(Pds::new);
		let dry = emulators()
			.into_iter()
			.map(|emulator| Pds::new(emulator).dry());
		for pds in repos.chain(dry) {
			let first = pds
				.put_record_if(
					&collection(),
					record("a", value!({ "n": 1 })),
					Precondition::Absent,
				)
				.await
				.unwrap();
			let refused = pds
				.put_record_if(
					&collection(),
					record("a", value!({ "n": 2 })),
					Precondition::Absent,
				)
				.await
				.unwrap_err();
			RecordConflict::of(&refused)
				.unwrap()
				.found
				.clone()
				.xpect_eq(Some(first.cid.clone()));
			let second = pds
				.put_record_if(
					&collection(),
					record("a", value!({ "n": 2 })),
					Precondition::Matches(first.cid.clone()),
				)
				.await
				.unwrap();
			pds.put_record_if(
				&collection(),
				record("a", value!({ "n": 3 })),
				Precondition::Matches(first.cid.clone()),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(first.cid.as_str())
			.xpect_contains(second.cid.as_str());
			pds.get_record(&collection(), &rkey("a"))
				.await
				.unwrap()
				.unwrap()
				.cid
				.xpect_eq(second.cid);
		}
	}
}
