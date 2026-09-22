use beet_core::prelude::*;

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
///
/// Not every mail credential is one of them: a comail api key is parked in
/// parameter store by the operator and read by the deploy verbs directly, so it
/// never transits state at all, and neither does the sovereign DKIM private
/// half (only its PUBLIC half is a variable). What state carries is what
/// terraform DERIVES, which for mail is the SES pair and nothing else.
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
		/// The one-deploy migration switch: read an existing PLAINTEXT state
		/// through OpenTofu's `unencrypted` fallback method and write it back
		/// encrypted. On for the apply that turns encryption on, off for every
		/// apply after, since a fallback left in place would also read a state
		/// somebody replaced with a plaintext one.
		migrate: bool,
	},
}

impl StateEncryption {
	/// Encrypt state and plan files with a passphrase read from `env_var`.
	pub fn passphrase(env_var: impl Into<SmolStr>) -> Self {
		Self::Passphrase {
			env_var: env_var.into(),
			migrate: false,
		}
	}

	/// The encryption with its migration switch set, see
	/// [`Passphrase::migrate`](Self::Passphrase).
	pub fn with_migrate(self, migrate: bool) -> Self {
		match self {
			Self::Passphrase { env_var, .. } => {
				Self::Passphrase { env_var, migrate }
			}
			Self::None => Self::None,
		}
	}

	/// The `terraform.encryption` block body, if enabled. `None` emits nothing,
	/// leaving the `terraform` block exactly as it is without this feature.
	///
	/// A target's `method` is a STATIC reference, which HCL's JSON syntax
	/// spells as the bare traversal (`"method.aes_gcm.main"`): wrapped in
	/// `${..}` it parses as a template expression and `tofu init` refuses it
	/// ("a single static variable reference is required"). The method's
	/// `keys` is an evaluated expression and keeps the interpolation.
	pub fn to_json(&self) -> Option<Value> {
		match self {
			Self::None => None,
			Self::Passphrase { migrate, .. } => {
				let mut methods = value!({
					"aes_gcm": {
						"main": { "keys": "${key_provider.pbkdf2.main}" }
					}
				});
				let mut state = value!({ "method": "method.aes_gcm.main" });
				if *migrate {
					methods
						.insert("unencrypted", value!({ "migrate": {} }))
						.ok();
					state
						.insert(
							"fallback",
							value!({
								"method": "method.unencrypted.migrate"
							}),
						)
						.ok();
				}
				Some(value!({
					"key_provider": {
						"pbkdf2": {
							"main": {
								"passphrase": (format!("${{var.{STATE_ENCRYPTION_VAR}}}")),
							}
						}
					},
					"method": methods,
					"state": state,
					"plan": { "method": "method.aes_gcm.main" },
				}))
			}
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
			Self::Passphrase { env_var, .. } => {
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

// native only: `tofu init` is the native cli, so every caller is
#[cfg(all(test, not(target_arch = "wasm32")))]
impl StateEncryption {
	/// Set the default passphrase variable for a test that reaches `tofu
	/// init` under an encrypted-by-default stack, when the environment (the
	/// runner's document, on a machine with the identity) carries none.
	pub(crate) fn ensure_test_passphrase() {
		use crate::prelude::Stack;
		if env_ext::var(Stack::DEFAULT_STATE_PASSPHRASE).is_err() {
			// SAFETY: test-only, a value no other test reads
			unsafe {
				env_ext::set_var(
					Stack::DEFAULT_STATE_PASSPHRASE,
					"test-passphrase",
				)
			}
			.ok();
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
			.unwrap()
			.into_json();
		json["key_provider"]["pbkdf2"]["main"]["passphrase"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.tf_state_passphrase}");
		// a static reference: the bare traversal, never an interpolation
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["plan"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["state"].get("fallback").xpect_none();
		json["method"].get("unencrypted").xpect_none();
	}

	/// The migration switch reads a plaintext state through the
	/// `unencrypted` fallback and nothing else changes.
	#[beet_core::test]
	fn migration_adds_the_unencrypted_fallback() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.with_migrate(true)
			.to_json()
			.unwrap()
			.into_json();
		json["method"]["unencrypted"]["migrate"]
			.as_object()
			.unwrap()
			.is_empty()
			.xpect_true();
		json["state"]["fallback"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.unencrypted.migrate");
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["plan"].get("fallback").xpect_none();
	}

	/// Native-only: wasm has no process environment to write to, so
	/// `set_var` there is not a failing assertion but an unimplemented
	/// platform call. The value resolution itself is exercised everywhere by
	/// [`passphrase_vars_errors_when_env_var_unset`].
	#[cfg(not(target_arch = "wasm32"))]
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
