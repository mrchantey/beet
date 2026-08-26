use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// The tofu variable name the passphrase is threaded through in generated
/// config. Internal: never appears outside a `var.` reference and a `-var`
/// invocation, see [`StateEncryption::vars`].
pub const STATE_ENCRYPTION_VAR: &str = "tf_state_passphrase";

/// OpenTofu client-side state (and plan) encryption.
/// https://opentofu.org/docs/language/state/encryption/
///
/// Terraform/OpenTofu state is plaintext by default, backend privacy
/// notwithstanding. A stack whose state carries secrets (a DB master
/// password, an SES SMTP credential, an admin password: anything an
/// `EnsureSecret`-style action feeds through a tofu variable) should enable
/// this, since those values land in state regardless of how they arrived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StateEncryption {
	/// State and plan files are plaintext (the default).
	#[default]
	None,
	/// PBKDF2-derived AES-GCM encryption. The passphrase is read from
	/// `env_var` at every tofu invocation that touches state ([`Self::vars`]),
	/// and is never written to `main.tf.json`: like an `EnsureSecret` value, it
	/// only ever exists as a `-var` at the point of invocation.
	Passphrase {
		/// Environment variable holding the passphrase, eg `TF_STATE_PASSPHRASE`.
		env_var: SmolStr,
	},
}

impl StateEncryption {
	/// Encrypt state and plan files with a passphrase read from `env_var`.
	pub fn passphrase(env_var: impl Into<SmolStr>) -> Self {
		Self::Passphrase {
			env_var: env_var.into(),
		}
	}

	/// The `terraform.encryption` block body, if enabled. `None` emits nothing,
	/// leaving the `terraform` block exactly as it is without this feature.
	pub fn to_json(&self) -> Option<Value> {
		match self {
			Self::None => None,
			Self::Passphrase { .. } => Some(json!({
				"key_provider": {
					"pbkdf2": {
						"main": {
							"passphrase": format!("${{var.{STATE_ENCRYPTION_VAR}}}"),
						}
					}
				},
				"method": {
					"aes_gcm": {
						"main": { "keys": "${key_provider.pbkdf2.main}" }
					}
				},
				"state": { "method": "${method.aes_gcm.main}" },
				"plan": { "method": "${method.aes_gcm.main}" },
			})),
		}
	}

	/// The `-var` pairs a live tofu invocation needs to read or write this
	/// stack's state: empty when encryption is off, else the passphrase read
	/// fresh from its environment variable.
	/// ## Errors
	/// - if enabled and the environment variable is unset
	pub fn vars(&self) -> Result<Vec<(SmolStr, SmolStr)>> {
		match self {
			Self::None => Ok(Vec::new()),
			Self::Passphrase { env_var } => {
				let value = env_ext::var(env_var.as_str()).map_err(|_| {
					bevyhow!(
						"state encryption is enabled but `{env_var}` is not set"
					)
				})?;
				Ok(vec![(STATE_ENCRYPTION_VAR.into(), value)])
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn none_emits_nothing() {
		StateEncryption::None.to_json().xpect_none();
		StateEncryption::None.vars().unwrap().xpect_empty();
	}

	#[beet_core::test]
	fn passphrase_emits_pbkdf2_aes_gcm() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.to_json()
			.unwrap();
		json["key_provider"]["pbkdf2"]["main"]["passphrase"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.tf_state_passphrase}");
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("${method.aes_gcm.main}");
	}

	#[beet_core::test]
	fn passphrase_vars_reads_its_env_var() {
		// SAFETY: test-only, single-threaded per-test env var scope is not
		// guaranteed, so use a name unlikely to collide with other tests.
		unsafe {
			std::env::set_var(
				"BEET_TEST_STATE_ENCRYPTION_PASSPHRASE",
				"super-secret",
			);
		}
		StateEncryption::passphrase("BEET_TEST_STATE_ENCRYPTION_PASSPHRASE")
			.vars()
			.unwrap()
			.xpect_eq(vec![(
				STATE_ENCRYPTION_VAR.into(),
				"super-secret".into(),
			)]);
		unsafe {
			std::env::remove_var("BEET_TEST_STATE_ENCRYPTION_PASSPHRASE");
		}
	}

	#[beet_core::test]
	fn passphrase_vars_errors_when_env_var_unset() {
		StateEncryption::passphrase("BEET_TEST_STATE_ENCRYPTION_MISSING")
			.vars()
			.unwrap_err();
	}
}
