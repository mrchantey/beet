//! The bytes a request's `media` posts reference, see [`RequestBlobs`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// The bytes of a request's `media` posts, read before its mapper maps so
/// neither mapper reads a store: a request is built in two steps, the
/// records answering its posts and then the bytes they reference read.
///
/// A post holding its bytes in memory, ie one this runtime made, answers
/// them itself; any other's are read from the window's [`ThreadStore`]. A
/// blob that cannot be read is left out, so the post reads as
/// [`Self::UNAVAILABLE`] and the turn goes on.
#[derive(Debug, Default, Clone)]
pub struct RequestBlobs(HashMap<Cid, Bytes>);

impl RequestBlobs {
	/// What a model and a view read in place of a file whose bytes are gone.
	pub const UNAVAILABLE: &'static str = "[file unavailable]";

	/// The blobs of `records` that no post holds in memory.
	pub fn missing(records: &ThreadRecords) -> Vec<BlobRef> {
		records
			.posts()
			.iter()
			.filter(|post| post.bytes().is_none())
			.filter_map(|post| match post.as_agent_post()? {
				AgentPost::Media(view) => view.blob(),
				_ => None,
			})
			.collect()
	}

	/// Read each of `missing` from the store of the window `thread`, a blob
	/// that cannot be read warned and left out.
	pub async fn read(
		thread: &AsyncEntity,
		missing: Vec<BlobRef>,
	) -> Result<Self> {
		let mut blobs = Self::default();
		if missing.is_empty() {
			return blobs.xok();
		}
		let store = thread
			.with_state::<Query<&ThreadStore>, _>(|entity, stores| {
				stores.get(entity).ok().cloned()
			})
			.await?;
		for blob in missing {
			let read = match &store {
				Some(store) => store.read_blob(&blob).await,
				None => Err(bevyhow!("the thread mounts no store")),
			};
			match read {
				Ok(bytes) => {
					blobs.0.insert(blob.cid, bytes);
				}
				Err(err) => {
					warn!("the file {} is unavailable: {err}", blob.cid)
				}
			}
		}
		blobs.xok()
	}

	/// Read the blobs the records of the window of the agent at `caller`
	/// reference that no post holds in memory.
	#[cfg(feature = "action")]
	pub async fn for_agent(caller: &AsyncEntity) -> Result<Self> {
		let (thread, missing) = caller
			.with_state::<ThreadQuery, _>(|entity, query| -> Result<_> {
				let (thread, _, records) = query.thread_and_records(entity)?;
				(thread, Self::missing(records)).xok()
			})
			.await??;
		Self::read(&caller.world().entity(thread), missing).await
	}

	/// The bytes of `post`, held in memory or read, `None` where they are
	/// unavailable.
	pub fn bytes(&self, post: &Post) -> Option<Bytes> {
		if let Some(bytes) = post.bytes() {
			return Some(bytes.clone());
		}
		match post.as_agent_post()? {
			AgentPost::Media(view) => self.0.get(&view.blob()?.cid).cloned(),
			_ => None,
		}
	}

	/// The base64 of `post`'s bytes, as a provider's json carries them.
	pub fn base64(&self, post: &Post) -> Option<String> {
		self.bytes(post).map(|bytes| {
			base64::Engine::encode(&base64::prelude::BASE64_STANDARD, bytes)
		})
	}
}
