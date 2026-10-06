use crate::prelude::*;

/// Every record type beet reads or writes, its own and foreign alike: the
/// record body and nothing else, so a type is exactly its lexicon.
///
/// The key a record lives at is its address, never part of its body, so it
/// travels beside the body: a read answers it in the record's uri, and a
/// write names it. A foreign record keyed by TID is written at the key it was
/// minted with, which the index remembering its natural key supplies; a beet
/// record derives its key from its identity components, which its collection
/// says how. A body read back from a repo therefore never pretends to know
/// where it lives.
///
/// `PartialEq` is the converge's comparison, so a type whose PDS fills a field
/// in later (a standard site document's `bskyPostRef`) is built with that
/// field from the index that remembers it rather than compared field by field
/// by hand. Serialized as the record body; the repo writes its `$type` as
/// [`COLLECTION`](Self::COLLECTION) when the type does not.
pub trait AtprotoRecord: Serialize + DeserializeOwned + PartialEq {
	/// The collection every record of this type lives in.
	const COLLECTION: Nsid;
}
