//! The opened side of a document: what this identity could read.

use crate::prelude::*;

/// What [`SecretsDocument::open`] yielded for one identity file: every
/// record of every group it could open, in one map, and each group named by
/// how it fared. Inserted on a `<Secrets>` entity once its document loads,
/// so a system reads a non-env secret by name without it ever touching the
/// environment; a group the identity cannot open is simply absent, which is
/// how an agent process sees only its own.
///
/// [`Debug`] redacts every value.
#[derive(Debug, Default, Clone, Component)]
pub struct OpenSecrets {
	/// Every opened record by name, from the sealed side.
	pub secrets: BTreeMap<SmolStr, Secret>,
	/// The groups that opened and verified.
	pub opened: Vec<SmolStr>,
	/// The groups none of this identity file's recipients is listed in.
	pub locked: Vec<SmolStr>,
	/// The groups that list one of this file's recipients but were sealed
	/// before it was added, so a member must `rekey`.
	pub pending: Vec<SmolStr>,
	/// The opened groups whose list changed since they were sealed: the next
	/// `set` or `rekey` seals to the new list.
	pub drifted: Vec<SmolStr>,
}

impl OpenSecrets {
	/// One record by name.
	pub fn get(&self, name: &str) -> Option<&Secret> { self.secrets.get(name) }

	/// Whether `group` opened.
	pub fn can_open(&self, group: &str) -> bool {
		self.opened.iter().any(|opened| opened == group)
	}

	/// The `EnvVar` records as pairs, from the sealed side.
	pub fn env_vars(&self) -> Vec<(SmolStr, SmolStr)> {
		self.secrets
			.values()
			.filter(|secret| secret.record.role == Some(SecretRole::EnvVar))
			.map(|secret| (secret.name.clone(), secret.value.clone()))
			.collect()
	}

	/// Set every `EnvVar` record into the process environment where it is
	/// not already set (the environment wins, then `.env`, then the
	/// document), answering how many landed.
	pub fn set_env_vars(&self) -> Result<usize> {
		let pairs = self
			.env_vars()
			.into_iter()
			.filter(|(key, _)| env_ext::var(key).is_err())
			.collect::<Vec<_>>();
		let count = pairs.len();
		env_ext::set_missing(pairs)?;
		count.xok()
	}

	/// The record names both this and `other` hold: two documents loaded
	/// into one world may not share one, since the second to load would
	/// silently shadow the first.
	pub fn overlap(&self, other: &Self) -> Vec<SmolStr> {
		self.secrets
			.keys()
			.filter(|name| other.secrets.contains_key(*name))
			.cloned()
			.collect()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn secret(name: &str, role: Option<SecretRole>) -> Secret {
		Secret {
			name: name.into(),
			group: "default".into(),
			value: format!("{name}-value").into(),
			record: SecretRecord { role, ..default() },
		}
	}

	fn open(secrets: impl IntoIterator<Item = Secret>) -> OpenSecrets {
		OpenSecrets {
			secrets: secrets
				.into_iter()
				.map(|secret| (secret.name.clone(), secret))
				.collect(),
			..default()
		}
	}

	#[crate::test]
	fn env_vars_are_the_env_var_records() {
		let open =
			open([secret("A", Some(SecretRole::EnvVar)), secret("B", None)]);
		open.env_vars()
			.xpect_eq(vec![("A".into(), "A-value".into())]);
	}

	/// Native only: the browser host has no mutable environment.
	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn set_env_vars_never_overwrites() {
		// SAFETY: test-only, a name no other test reads
		unsafe {
			env_ext::set_var("BEET_TEST_OPEN_SECRETS_SET", "already").unwrap();
		}
		let open = open([
			secret("BEET_TEST_OPEN_SECRETS_SET", Some(SecretRole::EnvVar)),
			secret("BEET_TEST_OPEN_SECRETS_NEW", Some(SecretRole::EnvVar)),
		]);
		open.set_env_vars().unwrap().xpect_eq(1);
		env_ext::var("BEET_TEST_OPEN_SECRETS_SET")
			.unwrap()
			.xpect_eq("already");
		env_ext::var("BEET_TEST_OPEN_SECRETS_NEW")
			.unwrap()
			.xpect_eq("BEET_TEST_OPEN_SECRETS_NEW-value");
		unsafe {
			env_ext::remove_var("BEET_TEST_OPEN_SECRETS_SET").unwrap();
			env_ext::remove_var("BEET_TEST_OPEN_SECRETS_NEW").unwrap();
		}
	}

	#[crate::test]
	fn overlap_names_shared_records() {
		let left = open([secret("A", None), secret("B", None)]);
		let right = open([secret("B", None), secret("C", None)]);
		left.overlap(&right).xpect_eq(vec![SmolStr::new("B")]);
		format!("{left:?}")
			.xpect_contains("<redacted>")
			.xnot()
			.xpect_contains("A-value");
	}
}
