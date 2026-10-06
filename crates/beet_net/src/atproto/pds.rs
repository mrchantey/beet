//! The provider-agnostic seam an account's repo lives behind.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bytes::Bytes;
use core::fmt;

/// One account's repo: a real PDS over xrpc, an emulated one in a
/// [`BlobStore`], or anything else downstream. A handle is scoped to the one
/// repo it was resolved for, exactly as a [`BlobStore`] is scoped to a bucket,
/// so every method takes a collection and an rkey and the provider composes
/// the address.
///
/// The erased-provider pattern: an `Arc<dyn PdsProvider>` cloned by every
/// consumer, landed on the declaring entity by `<AtprotoAccount/>`'s attach
/// and resolved by [`PdsQuery`]. [`Debug`] prints the provider and the did,
/// never a credential.
///
/// Bodies cross as [`AtprotoValue`], sealed in the protocol's data model, so
/// nothing a repo would refuse reaches one. The typed conveniences read and
/// write [`AtprotoRecord`] types, a write pairing the body with its rkey in an
/// [`Rkeyed`] and writing its `$type` as its collection when the type does
/// not, and [`converge`](Self::converge) makes a collection match what a
/// document declares.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async_ext::block_on(async {
/// let pds = Pds::temp();
/// let collection = Nsid::new_static("com.example.note");
/// let rkey = Rkey::parse("first").unwrap();
/// let written = pds
/// 	.put_record(&collection, Rkeyed::new(rkey.clone(), value!({ "text": "hi" }).into()))
/// 	.await?;
/// pds.get_record(&collection, &rkey)
/// 	.await?
/// 	.unwrap()
/// 	.strong_ref()
/// 	.xpect_eq(written);
/// # Ok::<_, BevyError>(())
/// # })
/// # .unwrap();
/// ```
#[derive(Clone, Component)]
pub struct Pds {
	provider: Arc<dyn PdsProvider>,
}

impl fmt::Debug for Pds {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Pds")
			.field("provider", &self.provider.id())
			.field("did", self.provider.did())
			.finish()
	}
}

impl Pds {
	/// The erased handle over `provider`.
	pub fn new(provider: impl PdsProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// An [`EmulatorPds`] in a fresh in-memory store, for a test.
	pub fn temp() -> Self { Self::new(EmulatorPds::temp()) }

	/// The provider's id, ie `emulator`, `xrpc`.
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// How a log names this repo: the provider and the did.
	pub fn describe(&self) -> String { self.provider.describe() }

	/// The account whose repo this is.
	pub fn did(&self) -> &Did { self.provider.did() }

	/// The address of `rkey` in `collection` of this repo.
	pub fn uri(&self, collection: &Nsid, rkey: &Rkey) -> AtUri {
		AtUri::new(self.did().clone(), collection.clone(), rkey.clone())
	}

	/// The record at `rkey`, `None` when there is none.
	pub async fn get_record(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
	) -> Result<Option<RecordEntry>> {
		self.provider
			.get_record(collection.clone(), rkey.clone())
			.await
	}

	/// Create or replace `record` at its rkey, answering the version
	/// written. A body with no `$type` is written as `collection`; one naming
	/// another collection is refused ([`AtprotoValue::into_record`]).
	pub async fn put_record(
		&self,
		collection: &Nsid,
		record: Rkeyed<AtprotoValue>,
	) -> Result<StrongRef> {
		self.provider
			.put_record(
				collection.clone(),
				record.try_map(|body| body.into_record(collection))?,
			)
			.await
	}

	/// Delete the record at `rkey`; one already gone is not an error.
	pub async fn delete_record(
		&self,
		collection: &Nsid,
		rkey: &Rkey,
	) -> Result {
		self.provider
			.delete_record(collection.clone(), rkey.clone())
			.await
	}

	/// Every record in `collection`, paginated to exhaustion, in rkey order.
	pub async fn list_records(
		&self,
		collection: &Nsid,
	) -> Result<Vec<RecordEntry>> {
		self.provider.list_records(collection.clone()).await
	}

	/// Upload `bytes` as a blob, answering the reference a record embeds to
	/// retain it.
	pub async fn upload_blob(
		&self,
		bytes: impl Into<Bytes>,
		mime_type: MediaType,
	) -> Result<BlobRef> {
		self.provider.upload_blob(bytes.into(), mime_type).await
	}

	/// The `T` at `rkey`, `None` when there is none.
	pub async fn get<T: AtprotoRecord>(
		&self,
		rkey: &Rkey,
	) -> Result<Option<RecordEntry<T>>> {
		self.get_record(&T::COLLECTION, rkey)
			.await?
			.map(RecordEntry::into_typed)
			.transpose()
	}

	/// Write `record` at its rkey.
	pub async fn put<T: AtprotoRecord>(
		&self,
		record: &Rkeyed<T>,
	) -> Result<StrongRef> {
		self.put_record(
			&T::COLLECTION,
			record.as_ref().try_map(AtprotoValue::from_serde)?,
		)
		.await
	}

	/// Every `T` in its collection, with its address and cid, in rkey order:
	/// the listing an index is rebuilt from. A record of the collection that
	/// does not read as a `T` is an error naming it.
	pub async fn list<T: AtprotoRecord>(&self) -> Result<Vec<RecordEntry<T>>> {
		self.list_records(&T::COLLECTION)
			.await?
			.into_iter()
			.map(RecordEntry::into_typed)
			.collect()
	}
}

/// An account's repo backend, see [`Pds`].
///
/// No `create_record`: the client mints every rkey, so a record never exists
/// without one and nothing waits on the PDS to mint. Reads need no
/// credential; a write a provider cannot authorize is an error naming what is
/// missing.
pub trait PdsProvider: 'static + Send + Sync {
	/// A boxed clone, so a default method can own the provider across an
	/// await.
	fn box_clone(&self) -> Box<dyn PdsProvider>;

	/// Stable provider discriminator, ie `emulator`, `xrpc`.
	fn id(&self) -> &'static str;

	/// Where the repo lives, for a log: the id and the did by default.
	fn describe(&self) -> String { format!("{} {}", self.id(), self.did()) }

	/// The account whose repo this is.
	fn did(&self) -> &Did;

	/// See [`Pds::get_record`].
	fn get_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result<Option<RecordEntry>>>;

	/// See [`Pds::put_record`]; `record` already carries its `$type`.
	fn put_record(
		&self,
		collection: Nsid,
		record: Rkeyed<AtprotoValue>,
	) -> SendBoxedFuture<Result<StrongRef>>;

	/// See [`Pds::delete_record`].
	fn delete_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result>;

	/// See [`Pds::list_records`].
	fn list_records(
		&self,
		collection: Nsid,
	) -> SendBoxedFuture<Result<Vec<RecordEntry>>>;

	/// See [`Pds::upload_blob`].
	fn upload_blob(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> SendBoxedFuture<Result<BlobRef>>;
}

/// One record as a repo stores it: its address, its cid and its body, the
/// `getRecord` answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordEntry<T = AtprotoValue> {
	/// The record's address.
	pub uri: AtUri,
	/// Its content id at this version.
	pub cid: Cid,
	/// Its body.
	pub value: T,
}

impl<T> RecordEntry<T> {
	/// The strong reference to this version.
	pub fn strong_ref(&self) -> StrongRef {
		StrongRef::new(self.uri.clone(), self.cid.clone())
	}

	/// The record's key.
	pub fn rkey(&self) -> &Rkey { self.uri.rkey() }

	/// The body paired with its key, ready to write back.
	pub fn into_rkeyed(self) -> Rkeyed<T> {
		Rkeyed::new(self.uri.rkey().clone(), self.value)
	}
}

impl RecordEntry {
	/// The body read as a `T`, an error naming the record when it is not one.
	pub fn into_typed<T: DeserializeOwned>(self) -> Result<RecordEntry<T>> {
		let uri = self.uri;
		RecordEntry {
			value: self.value.into_serde::<T>().map_err(|err| {
				bevyhow!(
					"{uri} does not read as a `{}`: {err}",
					core::any::type_name::<T>()
				)
			})?,
			uri,
			cid: self.cid,
		}
		.xok()
	}
}
