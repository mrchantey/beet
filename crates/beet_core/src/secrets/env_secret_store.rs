//! The launch's environment as a secret store.

use crate::prelude::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// The launch's environment as a [`SecretStore`], read-only: a secret's
/// label is its variable, so a record of the entry's secrets document is
/// read by the name the launch loaded it under (`BEET_SSH_HOST_KEY`). The
/// local meaning of a secret a deployed process reads from its stack's
/// store; a deployed process never reads a record from its environment.
///
/// Unscoped, since the environment is flat: [`for_scope`] is the same
/// store. Every write and the listing refuse, naming the verb that sets a
/// record, since the environment is the launch's and holds far more than
/// secrets.
///
/// [`for_scope`]: SecretStoreProvider::for_scope
#[derive(Debug, Default, Clone, Copy)]
pub struct EnvSecretStore;

impl EnvSecretStore {
	/// The provider id.
	pub const ID: &'static str = "env";

	/// The refusal every write and the listing answer with.
	fn read_only<T>(verb: &str) -> SendBoxedFuture<Result<T>> {
		let err = bevyhow!(
			"the launch's environment is a read-only secret store and cannot \
			{verb}: a record is set in the entry's secrets document \
			(`beet secrets/set NAME`), which the launch loads"
		);
		Box::pin(async move { Err(err) })
	}
}

impl SecretStoreProvider for EnvSecretStore {
	fn box_clone(&self) -> Box<dyn SecretStoreProvider> { Box::new(*self) }

	fn id(&self) -> &'static str { Self::ID }

	fn scope(&self) -> &SecretScope {
		static ROOT: SecretScope = SecretScope::ROOT;
		&ROOT
	}

	/// The environment is flat, so every scope is this one.
	fn for_scope(
		&self,
		_scope: &SecretScope,
	) -> Result<Box<dyn SecretStoreProvider>> {
		Ok(self.box_clone())
	}

	/// The variable, ie `$BEET_SSH_HOST_KEY`.
	fn address(&self, secret: &SecretRef) -> SmolStr {
		format!("${}", secret.label()).into()
	}

	fn get(
		&self,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let value = env_ext::var(secret.label()).ok().map(String::from);
		Box::pin(async move { value.xok() })
	}

	fn create(
		&self,
		_secret: SecretRef,
		_value: SmolStr,
		_meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		Self::read_only("create a secret")
	}

	fn overwrite(
		&self,
		_secret: SecretRef,
		_value: SmolStr,
		_meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		Self::read_only("overwrite a secret")
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
		Self::read_only("list its secrets")
	}

	fn delete(
		&self,
		_secrets: Vec<SecretRef>,
	) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
		Self::read_only("delete a secret")
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A loaded record reads by its name; a missing one is absence, and a
	/// caller requiring it is told the variable and never another value.
	#[crate::test]
	async fn reads_the_launch_records() {
		// SAFETY: test-only, variables no other test reads
		unsafe { env_ext::set_var("BEET_TEST_ENV_STORE", "hunter2") }.unwrap();
		let store = SecretStore::new(EnvSecretStore);
		store
			.get(&SecretRef::new("BEET_TEST_ENV_STORE"))
			.await
			.unwrap()
			.xpect_eq(Some("hunter2".into()));
		let missing = SecretRef::new("BEET_TEST_ENV_STORE_UNSET");
		store.get(&missing).await.unwrap().xpect_none();
		store
			.require(&missing, || "load the record".into())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("$BEET_TEST_ENV_STORE_UNSET")
			.xpect_contains("load the record")
			.xnot()
			.xpect_contains("hunter2");
		store
			.create(&missing, "x", None, SecretRoll::Remint)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("read-only");
		// SAFETY: as above
		unsafe { env_ext::remove_var("BEET_TEST_ENV_STORE") }.unwrap();
	}
}
