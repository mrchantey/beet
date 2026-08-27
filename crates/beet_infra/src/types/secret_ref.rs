//! Where a stack's secrets live in parameter store.
use crate::prelude::*;
use beet_core::prelude::*;

/// The SSM parameter one of a stack's secrets lives in, ie
/// `/beetmash/prod/db-password`: the `app--stage--label` composition with its
/// separators as slashes.
///
/// The slashes are the whole point. Parameter store treats them as a directory,
/// so every secret of one stack nests under a single prefix and an instance
/// role grants the lot in one statement rather than one per secret.
///
/// One type owns that composition and every end of the reference goes through
/// it: the block that reads the secret at boot, the compute block that grants
/// it, and [`EnsureSecret`] which creates it. A block spelling the string
/// itself is exactly the drift this exists to prevent, and it would land at
/// apply rather than at plan.
#[derive(
	Debug, Default, Clone, Get, Serialize, Deserialize, PartialEq, Eq, Reflect,
)]
pub struct SecretRef {
	/// The secret's label, ie the `db-password` in `/beetmash/prod/db-password`.
	label: SmolStr,
}

impl SecretRef {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
		}
	}

	/// The full parameter name, ie `/beetmash/prod/db-password`.
	pub fn name(&self, stack: &ResolvedStack) -> String {
		format!(
			"/{}",
			stack.resource_name(self.label.clone()).replace("--", "/")
		)
	}

	/// The directory every secret of `stack` sits under, ie `/beetmash/prod`,
	/// which is what an IAM statement grants with a single `/*`.
	pub fn prefix(stack: &ResolvedStack) -> String {
		// composed from a name rather than from the parts, so the prefix and
		// the names under it cannot disagree about the separator.
		Self::new("x")
			.name(stack)
			.rsplit_once('/')
			.map(|(prefix, _)| prefix.to_string())
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn stack() -> ResolvedStack {
		Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default())
	}

	/// The composition the live boot scripts and IAM policies already carry, so
	/// the strings are pinned: a renamed parameter is a box that boots without
	/// its database password.
	#[beet_core::test]
	fn composes_a_parameter_directory() {
		SecretRef::new("db-password")
			.name(&stack())
			.xpect_eq("/beetmash/prod/db-password");
		SecretRef::new("mail-admin-password")
			.name(&stack())
			.xpect_eq("/beetmash/prod/mail-admin-password");
	}

	/// The prefix is what makes one statement enough, so it must be the parent
	/// of every name and never the name itself.
	#[beet_core::test]
	fn the_prefix_is_the_directory_every_secret_sits_in() {
		let stack = stack();
		let prefix = SecretRef::prefix(&stack);
		prefix.as_str().xpect_eq("/beetmash/prod");
		SecretRef::new("db-password")
			.name(&stack)
			.starts_with(&format!("{prefix}/"))
			.xpect_true();
	}
}
