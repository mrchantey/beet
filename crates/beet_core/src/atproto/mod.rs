//! The AT Protocol's data model: the primitives a record is named, keyed and
//! referenced by, the trait every record type implements, and the provenance a
//! beet record and a derived file carry.
//!
//! Plain data with no network, so a compiler, a test and a runtime with no
//! connection all speak it. The repo these records live in is `beet_net`'s
//! `Pds`, whose module docs carry the converge's words.
//!
//! - [`Did`], an account; [`Nsid`], a collection or a lexicon def; [`Rkey`]
//!   and [`Tid`], a record's key; [`AtUri`], a record's address; [`Uri`],
//!   the lexicon's `uri` format verbatim
//! - [`Cid`], content addressing for bytes and records; [`StrongRef`] and
//!   [`BlobRef`], the two references a record holds
//! - [`AtprotoRecord`], one shape for every record type beet reads or writes,
//!   the body alone, and [`Rkeyed`], a body paired with the rkey it lives at
//! - [`AtprotoValue`], a value sealed in the data model, which is how a float
//!   reaches a repo that has none
//! - [`OpenUnion`], an object naming its own lexicon in `$type`, and
//!   [`SelfLabels`], the protocol's own content warnings
//! - [`Provenance`], what a beet record or a derived file was computed from
//!
//! # Words
//!
//! The protocol's own, used throughout beet. Beet's vocabulary (account, repo,
//! publication, derived file) is its glossary's.
//!
//! - **Rkey.** The record key a record is addressed by, the last segment of
//!   `at://<did>/<collection>/<rkey>`: 1 to 512 characters of
//!   `A-Za-z0-9._:~-`. A collection's lexicon says which kind it uses, `tid`
//!   for a record whose order matters, `nsid` for a lexicon schema,
//!   `literal:self` for a one-per-repo record like a profile. Beet mints every
//!   rkey client side on first write, so a record never exists without one and
//!   nothing waits on the PDS to mint.
//! - **TID.** The timestamp identifier most collections key by: a 64 bit
//!   integer, one zero bit then 53 bits of microseconds since the Unix epoch
//!   and 10 bits of clock id, written as 13 base32-sortable characters
//!   (`3mw72aaeuj22n`). Sortable by creation time and collision free without
//!   coordination, since a process picks its clock id once at random and its
//!   minter never repeats a value, using the last mint plus one when the clock
//!   has not moved.
//! - **Strong ref.** A `{uri, cid}` pair pinning one exact version of a
//!   record, where the uri alone names whatever sits at that address now. A
//!   cid is the content hash of the record as the protocol encodes it, so a
//!   strong ref goes stale the moment the record is written again.
//! - **Blob.** Here a blob is content addressed rather than path keyed:
//!   `uploadBlob` answers with a `$type: "blob"` object carrying a cid, a mime
//!   type and a size, and a record embeds that object. The PDS keeps a blob
//!   only while some current record references it, so the reference is the
//!   retention and a bare cid is not one. Beet's own blob, bytes under a path,
//!   is the glossary's.
mod at_uri;
#[cfg(feature = "serde")]
mod atproto_record;
mod atproto_value;
mod blob_ref;
mod cid;
mod did;
mod nsid;
mod open_union;
mod provenance;
mod rkey;
mod rkeyed;
mod self_labels;
mod strong_ref;
mod tid;
mod uri;
pub use at_uri::*;
#[cfg(feature = "serde")]
pub use atproto_record::*;
pub use atproto_value::*;
pub use blob_ref::*;
pub use cid::*;
pub use did::*;
pub use nsid::*;
pub use open_union::*;
pub use provenance::*;
pub use rkey::*;
pub use rkeyed::*;
pub use self_labels::*;
pub use strong_ref::*;
pub use tid::*;
pub use uri::*;

/// The conversions every string primitive here shares: its text, `Display`,
/// `FromStr` and `TryFrom<SmolStr>` through its validating `parse`, and the
/// serde round trip through that text so a deserialized value holds the
/// invariants.
macro_rules! string_primitive {
	($ty:ident) => {
		impl $ty {
			/// The text this value is written as.
			pub fn as_str(&self) -> &str { self.0.as_str() }
		}

		impl core::fmt::Display for $ty {
			fn fmt(
				&self,
				formatter: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				formatter.write_str(self.0.as_str())
			}
		}

		impl AsRef<str> for $ty {
			fn as_ref(&self) -> &str { self.0.as_str() }
		}

		impl core::str::FromStr for $ty {
			type Err = BevyError;
			fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
		}

		impl TryFrom<SmolStr> for $ty {
			type Error = BevyError;
			fn try_from(text: SmolStr) -> Result<Self> { Self::parse(&text) }
		}

		impl From<$ty> for SmolStr {
			fn from(value: $ty) -> SmolStr { value.0 }
		}
	};
}
pub(crate) use string_primitive;
