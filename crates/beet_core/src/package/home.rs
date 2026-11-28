//! [`Home`], the identity of an account's home record.
use crate::prelude::*;

/// `org.beet.core#home`, the identity of the home record (`org.beet.home`,
/// rkey `self`, in the public repo): the account's home package, by name. A
/// pointer by name and never by cid, so it cannot go stale: `@handle` alone
/// resolves the handle, its did, this record, then the package's `latest`
/// release.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, LexiconDef, Serialize, Deserialize)]
pub struct Home {
	/// The home package.
	pub package: PackageName,
}
