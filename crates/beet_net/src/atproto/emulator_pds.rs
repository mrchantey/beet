//! A repo laid out in any [`BlobStore`], doing a PDS's duties on write.
use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A [`PdsProvider`] over any [`BlobStore`]: the repo a test writes to, and
/// the one a repo that never leaves the machine (or lives in a bucket) is.
///
/// Authoritative on the layout, which is plain enough to read in a file
/// browser and to mirror with `BlobSync`:
///
/// ```text
/// records/<collection>/<rkey>.json   a record body, the protocol's json form
/// blobs/<cid>                        a blob's bytes
/// ```
///
/// The `.json` suffix names the format to every store that picks a media type
/// by extension; an rkey may hold a dot, so exactly one suffix is stripped.
///
/// And authoritative on the PDS's duties on write, so a repo here and one on a
/// real PDS cannot drift apart:
///
/// - the record cid is computed over canonical DAG-CBOR under CIDv1, exactly
///   as a PDS computes it (`dag_cbor_ext`), so a record's strong ref and any
///   rkey derived from it are the same in a bucket and on the network;
/// - a record's `$type` must be its collection, and every blob it references
///   must have been uploaded;
/// - a blob is retained exactly while a current record references it: a write
///   or delete that drops a record's last reference to a blob deletes the
///   blob, and [`sweep`](Self::sweep) collects uploads no record ever took up,
///   the PDS's periodic garbage collection;
/// - a listing is per collection, in rkey order.
#[derive(Debug, Clone)]
pub struct EmulatorPds {
	did: Did,
	store: BlobStore,
}

impl EmulatorPds {
	/// The provider id.
	pub const ID: &'static str = "emulator";
	/// The did a test repo answers for when none is given.
	pub const TEST_DID: &'static str = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";

	const RECORDS: &'static str = "records";
	const BLOBS: &'static str = "blobs";
	const SUFFIX: &'static str = ".json";

	/// The repo of `did` laid out in `store`.
	pub fn new(did: Did, store: BlobStore) -> Self { Self { did, store } }

	/// A repo in a fresh in-memory store, answering for [`Self::TEST_DID`].
	pub fn temp() -> Self {
		Self::new(
			Did::parse(Self::TEST_DID).unwrap_or_default(),
			BlobStore::temp(),
		)
	}

	/// The store the repo is laid out in.
	pub fn store(&self) -> &BlobStore { &self.store }

	/// Delete every blob no current record references, answering their cids:
	/// what a PDS does to an upload no record took up.
	pub async fn sweep(&self) -> Result<Vec<Cid>> {
		let blobs = self
			.store
			.list_dir(&RelPath::new(Self::BLOBS))
			.await?
			.files
			.into_iter()
			.map(|file| Cid::parse(&file))
			.collect::<Result<HashSet<_>>>()?;
		self.release(blobs).await
	}

	fn record_path(collection: &Nsid, rkey: &Rkey) -> RelPath {
		RelPath::new(format!(
			"{}/{collection}/{rkey}{}",
			Self::RECORDS,
			Self::SUFFIX
		))
	}

	fn blob_path(cid: &Cid) -> RelPath {
		RelPath::new(format!("{}/{cid}", Self::BLOBS))
	}

	/// The body stored at `path`, `None` when there is none.
	async fn read(&self, path: &RelPath) -> Result<Option<Value>> {
		if !self.store.exists(path).await? {
			return Ok(None);
		}
		let bytes = self.store.get(path).await?;
		Value::from_json(serde_json::from_slice(&bytes)?)
			.xmap(Some)
			.xok()
	}

	async fn get(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
	) -> Result<Option<RecordEntry>> {
		self.read(&Self::record_path(collection, rkey))
			.await?
			.map(|value| self.entry(collection, rkey, value))
			.transpose()
	}

	fn entry(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
		value: Value,
	) -> Result<RecordEntry> {
		RecordEntry {
			uri: AtUri::new(self.did.clone(), collection.clone(), rkey.clone()),
			cid: dag_cbor_ext::record_cid(&value)?,
			value,
		}
		.xok()
	}

	async fn put(
		&self,
		collection: Nsid,
		rkey: Rkey,
		record: Value,
	) -> Result<StrongRef> {
		let record = RecordEntry::typed_body(&collection, record)?;
		let cid = dag_cbor_ext::record_cid(&record)?;
		let referenced = Self::blob_refs(&record);
		for blob in &referenced {
			if !self.store.exists(&Self::blob_path(blob)).await? {
				bevybail!(
					"{} references blob {blob}, which was never uploaded to \
					 this repo",
					AtUri::new(self.did.clone(), collection, rkey)
				);
			}
		}
		let path = Self::record_path(&collection, &rkey);
		let dropped = self.dropped_blobs(&path, &referenced).await?;
		self.store
			.insert(&path, serde_json::to_vec(&record.into_json())?)
			.await?;
		self.release(dropped).await?;
		StrongRef::new(AtUri::new(self.did.clone(), collection, rkey), cid)
			.xok()
	}

	async fn delete(&self, collection: Nsid, rkey: Rkey) -> Result {
		let path = Self::record_path(&collection, &rkey);
		let dropped = self.dropped_blobs(&path, &HashSet::default()).await?;
		if self.store.exists(&path).await? {
			self.store.remove(&path).await?;
		}
		self.release(dropped).await?;
		Ok(())
	}

	async fn list(&self, collection: &Nsid) -> Result<Vec<RecordEntry>> {
		let mut rkeys = self
			.store
			.list_dir(&RelPath::new(format!("{}/{collection}", Self::RECORDS)))
			.await?
			.files
			.into_iter()
			.filter_map(|file| file.strip_suffix(Self::SUFFIX).map(Rkey::parse))
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
		path: &RelPath,
		kept: &HashSet<Cid>,
	) -> Result<HashSet<Cid>> {
		self.read(path)
			.await?
			.map(|old| Self::blob_refs(&old))
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
		let records = self
			.store
			.list()
			.await?
			.into_iter()
			.filter(|path| path.as_str().starts_with(Self::RECORDS));
		for path in records {
			if let Some(record) = self.read(&path).await? {
				for cid in Self::blob_refs(&record) {
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

	/// Every blob `value` references, anywhere in it: the PDS retains by
	/// structure, so a reference nested in any field counts.
	fn blob_refs(value: &Value) -> HashSet<Cid> {
		let mut found = HashSet::default();
		Self::collect_blob_refs(value, &mut found);
		found
	}

	fn collect_blob_refs(value: &Value, found: &mut HashSet<Cid>) {
		match value {
			Value::Map(map) => {
				if map.get("$type").ok().and_then(|ty| ty.as_str().ok())
					== Some(BlobRef::TYPE)
					&& let Ok(blob) = value.clone().into_serde::<BlobRef>()
				{
					found.insert(blob.cid);
				}
				for (_, child) in map {
					Self::collect_blob_refs(child, found);
				}
			}
			Value::List(items) => {
				for item in items {
					Self::collect_blob_refs(item, found);
				}
			}
			_ => {}
		}
	}
}

impl PdsProvider for EmulatorPds {
	fn box_clone(&self) -> Box<dyn PdsProvider> { Box::new(self.clone()) }

	fn id(&self) -> &'static str { Self::ID }

	fn describe(&self) -> String {
		format!("{} {} in {}", Self::ID, self.did, self.store.describe())
	}

	fn did(&self) -> &Did { &self.did }

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
		rkey: Rkey,
		record: Value,
	) -> SendBoxedFuture<Result<StrongRef>> {
		let this = self.clone();
		Box::pin(async move { this.put(collection, rkey, record).await })
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn collection() -> Nsid { Nsid::new_static("com.example.card") }
	fn rkey(key: &str) -> Rkey { Rkey::parse(key).unwrap() }

	/// A card holding `blob`, the shape every retention case writes.
	fn card(blob: &BlobRef) -> Value {
		value!({ "image": (Value::from_serde(blob).unwrap()) })
	}

	/// The layout is the documented one, and the cid a written record answers
	/// is the one a read recomputes.
	#[beet_core::test]
	async fn lays_out_records_and_blobs() {
		let emulator = EmulatorPds::temp();
		let pds = Pds::new(emulator.clone());
		let blob = pds.upload_blob("png", MediaType::Png).await.unwrap();
		let written = pds
			.put_record(&collection(), &rkey("a"), card(&blob))
			.await
			.unwrap();
		let mut paths = emulator.store().list().await.unwrap();
		paths.sort();
		paths.xpect_eq(vec![
			RelPath::new(format!("blobs/{}", blob.cid)),
			RelPath::new("records/com.example.card/a.json"),
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

	/// A record's `$type` is its collection, and a blob it names must exist.
	#[beet_core::test]
	async fn refuses_what_a_pds_refuses() {
		let pds = Pds::temp();
		pds.put_record(
			&collection(),
			&rkey("a"),
			value!({ "$type": "com.example.other" }),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("declares `$type");
		let blob = BlobRef::of(b"never uploaded", MediaType::Png);
		pds.put_record(&collection(), &rkey("a"), card(&blob))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("never uploaded");
	}

	/// A blob lives exactly while a current record references it.
	#[beet_core::test]
	async fn retains_referenced_blobs() {
		let emulator = EmulatorPds::temp();
		let pds = Pds::new(emulator.clone());
		let exists = async |blob: &BlobRef| {
			emulator
				.store()
				.exists(&RelPath::new(format!("blobs/{}", blob.cid)))
				.await
				.unwrap()
		};
		let first = pds.upload_blob("one", MediaType::Png).await.unwrap();
		let second = pds.upload_blob("two", MediaType::Png).await.unwrap();
		// two records share the first blob
		pds.put_record(&collection(), &rkey("a"), card(&first))
			.await
			.unwrap();
		pds.put_record(&collection(), &rkey("b"), card(&first))
			.await
			.unwrap();
		// replacing one keeps the blob the other still holds
		pds.put_record(&collection(), &rkey("a"), card(&second))
			.await
			.unwrap();
		exists(&first).await.xpect_true();
		// deleting the last holder releases it
		pds.delete_record(&collection(), &rkey("b")).await.unwrap();
		exists(&first).await.xpect_false();
		exists(&second).await.xpect_true();
		// an upload no record takes up is swept
		let orphan = pds.upload_blob("three", MediaType::Png).await.unwrap();
		emulator
			.sweep()
			.await
			.unwrap()
			.xpect_eq(vec![orphan.cid.clone()]);
		exists(&orphan).await.xpect_false();
		exists(&second).await.xpect_true();
	}

	#[beet_core::test]
	async fn lists_a_collection_in_rkey_order() {
		let pds = Pds::temp();
		for key in ["c", "a", "b"] {
			pds.put_record(&collection(), &rkey(key), value!({ "key": key }))
				.await
				.unwrap();
		}
		pds.put_record(
			&Nsid::new_static("com.example.other"),
			&rkey("z"),
			value!({}),
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
