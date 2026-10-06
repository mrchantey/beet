use crate::prelude::*;

/// Every record type beet reads or writes, its own and foreign alike.
///
/// `PartialEq` is the converge's comparison, so a type whose PDS fills a field
/// in later (a standard site document's `bskyPostRef`) is built with that
/// field from the index that remembers it rather than compared field by field
/// by hand. `rkey` is total: a beet record derives it from its identity
/// components; a foreign record keyed by TID carries the one it was minted
/// with, minted client side on first write and remembered by the index. A
/// converge is therefore get, compare, put, and lists only to find orphans or
/// rebuild a lost index.
///
/// Serialized as the record body; the repo writes its `$type` as
/// [`COLLECTION`](Self::COLLECTION) when the type does not.
pub trait AtprotoRecord: Serialize + DeserializeOwned + PartialEq {
	/// The collection every record of this type lives in.
	const COLLECTION: Nsid;
	/// The key this record lives at.
	fn rkey(&self) -> Rkey;
}
