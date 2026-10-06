use crate::prelude::*;

/// The protocol's strong reference, `com.atproto.repo.strongRef`: one exact
/// version of a record, where the [`AtUri`] alone names whatever sits at that
/// address now. Goes stale the moment the record is written again.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct StrongRef {
	/// The record's address.
	pub uri: AtUri,
	/// The record's content id at that version.
	pub cid: Cid,
}

impl StrongRef {
	/// The reference to `uri` at `cid`.
	pub fn new(uri: AtUri, cid: Cid) -> Self { Self { uri, cid } }
}

impl core::fmt::Display for StrongRef {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(formatter, "{} ({})", self.uri, self.cid)
	}
}
