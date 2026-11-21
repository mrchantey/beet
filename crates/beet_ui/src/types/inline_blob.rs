//! [`InlineBlob`], an image source's bytes held on the tree that names it.
use beet_core::prelude::*;

/// One source's media, fetched by a media resolve step and held on the
/// source attribute's entity (or, for the cover image, the render root) for
/// the life of the tree, so a render writes a correct [`BlobRef`] the first
/// time and a publish uploads straight from the tree it rendered.
///
/// An ordinary component, so a later step hooks in by query or observer, ie
/// an image optimiser or a size check against a PDS's blob limit.
///
/// Derived state, so no saver writes it: a scene carries the source's url as
/// the page has it, never the bytes fetched from it.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Derived)]
pub struct InlineBlob {
	/// The bytes' content id under the raw codec, what `uploadBlob` answers.
	pub cid: Cid,
	/// The resolved source, kept for errors and deduplication.
	pub link: Url,
	/// The fetched bytes, one shared buffer every attribute naming this source
	/// holds.
	pub bytes: MediaBytes,
}

impl InlineBlob {
	/// The blob `bytes` fetched from `link` are.
	pub fn new(link: Url, bytes: MediaBytes) -> Self {
		Self {
			cid: Cid::raw(&bytes),
			link,
			bytes,
		}
	}

	/// The reference a record embeds for these bytes, computed with no
	/// network: the cid, the media type and the size.
	pub fn blob_ref(&self) -> BlobRef {
		BlobRef {
			cid: self.cid.clone(),
			mime_type: self.bytes.media_type().clone(),
			size: self.bytes.len() as u64,
		}
	}
}
