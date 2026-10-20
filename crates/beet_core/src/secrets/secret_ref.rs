//! How a secret is named: its label, and the scope a store nests it under.

use crate::prelude::*;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use heck::ToKebabCase;

/// One secret by label, ie `db-password`: the key every [`SecretStore`]
/// method takes, whatever provider holds the value. A store composes its own
/// address from the label and its [`SecretScope`]
/// ([`SecretStore::address`]), so a label never couples to a provider and
/// an export restores into another provider or region unchanged.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Get, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SecretRef {
	/// The secret's label, ie the `db-password` in
	/// `/beetmash/prod/db-password`.
	label: SmolStr,
}

impl SecretRef {
	/// The secret labelled `label`.
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
		}
	}
}

impl fmt::Display for SecretRef {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.label)
	}
}

/// What a [`SecretStore`] is scoped to, as path segments: `beetmash/prod` for
/// a stack, the app then the stage. A handle is scoped exactly as a blob
/// store is scoped to a bucket and prefix, and another scope of the same
/// provider is [`SecretStore::for_scope`] away.
///
/// [`path`](Self::path) is the hierarchical composition,
/// `/beetmash/prod/db-password`, every segment and the label kebab-cased:
/// parameter store treats it as a directory, so every secret of one scope
/// nests under [`prefix`](Self::prefix) and an IAM grant names the lot in
/// one statement. It lives here rather than with a provider because the
/// grants a compute lowers name the same path, and a block spelling the
/// string itself is exactly the drift this prevents.
///
/// ```
/// # use beet_core::prelude::*;
/// let scope = SecretScope::new(["beet_site", "prod"]);
/// scope.to_string().xpect_eq("beet-site/prod");
/// scope.prefix().xpect_eq("/beet-site/prod");
/// scope
/// 	.path(&SecretRef::new("ssh-host-key"))
/// 	.xpect_eq("/beet-site/prod/ssh-host-key");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct SecretScope(Vec<SmolStr>);

impl SecretScope {
	/// The empty scope, of a flat store like the launch's environment.
	pub const ROOT: Self = Self(Vec::new());

	/// The scope of `segments`, each kebab-cased.
	pub fn new(segments: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
		Self(
			segments
				.into_iter()
				.map(|segment| SmolStr::new(segment.as_ref().to_kebab_case()))
				.collect(),
		)
	}

	/// The segments, outermost first.
	pub fn segments(&self) -> &[SmolStr] { &self.0 }

	/// The directory every secret of this scope sits under, ie
	/// `/beetmash/prod`, which an IAM statement grants with a single `/*`.
	pub fn prefix(&self) -> String {
		self.0.iter().fold(String::new(), |mut prefix, segment| {
			prefix.push('/');
			prefix.push_str(segment);
			prefix
		})
	}

	/// The hierarchical address of `secret` in this scope, ie
	/// `/beetmash/prod/db-password`: a parameter store name, and what a
	/// grant on it names.
	pub fn path(&self, secret: &SecretRef) -> String {
		format!("{}/{}", self.prefix(), secret.label().to_kebab_case())
	}
}

impl fmt::Display for SecretScope {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0.join("/"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The composition the live grants and parameters already carry, so the
	/// strings are pinned: a renamed parameter is a box that boots without
	/// its secret. The prefix is the parent of every path, never a path.
	#[crate::test]
	fn composes_a_parameter_directory() {
		let scope = SecretScope::new(["beetmash", "prod"]);
		scope
			.path(&SecretRef::new("db-password"))
			.xpect_eq("/beetmash/prod/db-password");
		scope
			.path(&SecretRef::new("mail-admin-password"))
			.xpect_eq("/beetmash/prod/mail-admin-password");
		scope.prefix().xpect_eq("/beetmash/prod");
		SecretScope::new(["beet_infra", "dev"])
			.path(&SecretRef::new("db_password"))
			.xpect_eq("/beet-infra/dev/db-password");
		SecretScope::default().prefix().xpect_eq("");
	}
}
