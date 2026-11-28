//! [`RepoScope`]: which repo of an account a record lives in, its public one
//! or a member's within one space.
use crate::prelude::*;

/// One space of an account: its type, an NSID naming the modality and
/// resolving to a declaration that defines no schema, and its key.
///
/// A package's space is of type [`SpaceRef::PACKAGE`] keyed by the package's
/// name, `at://<did>/space/org.beet.space.package/<name>`.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub struct SpaceRef {
	/// The modality, ie [`SpaceRef::PACKAGE`].
	pub r#type: Nsid,
	/// The space's key within its type, keyed as an rkey is since it is a
	/// segment of every address in the space: a package's name for a package
	/// space.
	pub key: Rkey,
}

impl SpaceRef {
	/// The type every package's space shares, so one consent covers every
	/// package of an account.
	pub const PACKAGE: Nsid = Nsid::new_static("org.beet.space.package");

	/// The space of `type` keyed `key`.
	pub fn new(r#type: Nsid, key: Rkey) -> Self { Self { r#type, key } }

	/// The space of `type` keyed by the text `key`.
	///
	/// # Errors
	/// Errors when `key` is empty or holds a character an rkey may not.
	pub fn parse(r#type: Nsid, key: &str) -> Result<Self> {
		Rkey::parse(key)
			.map_err(|err| {
				bevyhow!("`{key}` is no space key, keyed as an rkey is: {err}")
			})?
			.xmap(|key| Self::new(r#type, key))
			.xok()
	}

	/// The space of the package `name`.
	///
	/// # Errors
	/// Errors as [`parse`](Self::parse).
	pub fn package(name: &str) -> Result<Self> { Self::parse(Self::PACKAGE, name) }
}

impl core::fmt::Display for SpaceRef {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}/{}", self.r#type, self.key)
	}
}

/// Which repo of an account: its public repo, or one member's repo within one
/// space. A record is addressed within its scope, so a handle scoped to either
/// reads and writes the same way and never learns which it holds.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum RepoScope {
	/// The account's public repo: what was published.
	#[default]
	Public,
	/// `author`'s repo within `space`, the account's own when `author` is the
	/// account.
	Space {
		/// The space.
		space: SpaceRef,
		/// The member who wrote the records.
		author: Did,
	},
}

impl RepoScope {
	/// The literal segment an at uri carries before a space's type, telling a
	/// space record from a public one.
	pub const SPACE_SEGMENT: &'static str = "space";

	/// `author`'s repo within `space`.
	pub fn space(space: SpaceRef, author: Did) -> Self {
		Self::Space { space, author }
	}

	/// The space this scope is within, `None` for the public repo.
	pub fn space_ref(&self) -> Option<&SpaceRef> {
		match self {
			Self::Public => None,
			Self::Space { space, .. } => Some(space),
		}
	}
}
