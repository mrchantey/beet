//! A file computed from declared sources and stored beside the provenance
//! naming them: an index, a generated icon, a compiled scene.
//!
//! There is no `DerivedFile` type. A derived file is an ordinary entity-body
//! document that carries [`Provenance`], which is the same component every beet
//! record carries, so a reader needs one concept and a writer one component.
//!
//! # Fresh
//!
//! Fresh means every source's digest is unchanged and the `ttl` has not expired.
//! A source is a file by path and cid, or a record by strong ref; the `ttl`
//! covers only what cannot be seen from here, an upstream nothing local hashes.
//! `ensure()` regenerates a stale or missing file and does nothing otherwise, so
//! running it twice costs one comparison.
//!
//! # Where one is stored, and why
//!
//! The store is resolved by ancestry like any other, and the choice is about who
//! reviews the file rather than about how it was made:
//!
//! - Git, under the package's `derived/` directory, for a file that CANNOT be
//!   reproduced from its sources alone and whose change a human should read in a
//!   diff. The standard site index is the case: it remembers which rkey each
//!   record was minted at, and nothing but a listing of the PDS could recover
//!   that. The `derived/` directory is what keeps it from being mistaken for a
//!   source, and the router maps it explicitly.
//! - A bucket, for a file that is reproducible but binary, where a diff reads as
//!   noise. The icon set is the case: every size falls out of the one source
//!   image, so the bucket holds the current answer and the source image's digest
//!   is the whole of its provenance.
//!
//! A reproducible non-binary file is a cache: ignored locally and published
//! under a derived prefix, never committed.
//!
//! # A stale file still serves
//!
//! A consumer reads a derived file and never asks how it was made, so a stale
//! one is served with a warning naming the verb that refreshes it. Refusing to
//! serve would turn a missed regeneration into an outage, and the file is by
//! definition a copy of something that still exists.
use crate::prelude::*;

/// `org.beet.core#provenance`, required on every beet record: when it was
/// first written and, for a derived document, what it was computed from.
///
/// A compiled record names its inputs elsewhere (a package its source branch,
/// a scene its content key), so its `sources` are empty. A derived file lists
/// each input with the digest it had, and its freshness is the module docs'.
/// `created` is written as ISO 8601, the form an atproto `datetime` takes and
/// the one a reviewer reads in a committed file's diff.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Provenance {
	/// When this was first written; on a scene record the first compile that
	/// produced this content, on a derived file the derivation that wrote it.
	#[cfg_attr(feature = "serde", serde(with = "timestamp_iso8601"))]
	pub created: Timestamp,
	/// The inputs a derived document was computed from, each with the digest
	/// it had.
	#[cfg_attr(feature = "serde", serde(default))]
	pub sources: Vec<ProvenanceSource>,
	/// How long the derivation holds for an input nothing local can hash.
	#[cfg_attr(feature = "serde", serde(default))]
	pub ttl: Option<Duration>,
}

impl Provenance {
	/// A provenance created now from `sources`, with no ttl.
	pub fn new(sources: impl IntoIterator<Item = ProvenanceSource>) -> Self {
		Self {
			created: Timestamp::now(),
			sources: sources.into_iter().collect(),
			ttl: None,
		}
	}

	/// This provenance holding for `ttl`, or for ever when `None`.
	pub fn with_ttl(mut self, ttl: Option<Duration>) -> Self {
		self.ttl = ttl;
		self
	}

	/// Whether the ttl has run out at `now`; never without one.
	pub fn expired(&self, now: Timestamp) -> bool {
		self.ttl.is_some_and(|ttl| self.created + ttl <= now)
	}
}

/// One input of a derived document, and the digest it had.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub enum ProvenanceSource {
	/// A file by its path in the store it was read from, and its content id.
	File {
		/// The file's path.
		path: RelPath,
		/// Its content id when the derivation read it.
		cid: Cid,
	},
	/// A record at the version the derivation read.
	Record(StrongRef),
}

impl ProvenanceSource {
	/// The file at `path` as read, its digest the raw cid of `bytes`.
	pub fn file(path: impl Into<RelPath>, bytes: &[u8]) -> Self {
		Self::File {
			path: path.into(),
			cid: Cid::raw(bytes),
		}
	}

	/// The digest this source had when the derivation read it.
	pub fn cid(&self) -> &Cid {
		match self {
			Self::File { cid, .. } => cid,
			Self::Record(strong_ref) => &strong_ref.cid,
		}
	}
}

impl core::fmt::Display for ProvenanceSource {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::File { path, .. } => write!(f, "file `{path}`"),
			Self::Record(strong_ref) => {
				write!(f, "record `{}`", strong_ref.uri)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn expires_after_its_ttl() {
		let provenance = Provenance {
			created: Timestamp::from_secs(100),
			sources: default(),
			ttl: Some(Duration::from_secs(10)),
		};
		provenance.expired(Timestamp::from_secs(109)).xpect_false();
		provenance.expired(Timestamp::from_secs(110)).xpect_true();
		// no ttl holds for ever
		provenance
			.with_ttl(None)
			.expired(Timestamp::from_secs(i64::MAX / 1_000))
			.xpect_false();
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn created_reads_as_iso8601() {
		let provenance = Provenance {
			created: Timestamp::from_secs(0),
			sources: vec![ProvenanceSource::file("logo.png", b"logo")],
			ttl: None,
		};
		let json = MediaType::Json.serialize(&provenance).unwrap();
		str::from_utf8(&json)
			.unwrap()
			.xpect_contains("\"created\":\"1970-01-01T00:00:00.000Z\"");
		MediaType::Json
			.deserialize::<Provenance>(&json)
			.unwrap()
			.xpect_eq(provenance);
	}
}
