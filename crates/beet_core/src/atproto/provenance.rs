//! What a beet record or a derived file was computed from.
use crate::prelude::*;

/// `org.beet.core#provenance`, required on every beet record: when it was
/// first written and, for a derived document, what it was computed from.
///
/// A compiled record names its inputs elsewhere (a package its source branch,
/// a snapshot its content key), so its `sources` are empty. A derived file is
/// an entity-body document carrying this component, listing each input with
/// the digest it had, and `ttl` covers an input whose change cannot be seen
/// from here, ie records a PDS minted. Fresh means every digest unchanged and
/// the ttl unexpired.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Provenance {
	/// When this was first written; on a compiled snapshot the first compile
	/// that produced this content.
	pub created: Timestamp,
	/// The inputs a derived document was computed from, each with the digest
	/// it had.
	#[cfg_attr(feature = "serde", serde(default))]
	pub sources: Vec<ProvenanceSource>,
	/// How long the derivation holds for an input nothing local can hash.
	#[cfg_attr(feature = "serde", serde(default))]
	pub ttl: Option<Duration>,
}

/// One input of a derived document, and the digest it had.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub enum ProvenanceSource {
	/// A file by its path in the store it was read from, and its content id.
	Blob {
		/// The file's path.
		path: RelPath,
		/// Its content id when the derivation read it.
		cid: Cid,
	},
	/// A record at the version the derivation read.
	Record(StrongRef),
}
