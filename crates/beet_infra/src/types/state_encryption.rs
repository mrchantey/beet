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
		/// The plaintext bridge this render crosses, [`StateBridge::None`] in
		/// the steady state.
		bridge: StateBridge,
	},
}

/// How one state write crosses between plaintext and encrypted: OpenTofu's
/// `unencrypted` method beside `aes_gcm`, one as the `method` a write uses
/// and the other as the `fallback` a read may take. Never left in place:
/// a fallback that stays would also read a state somebody replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateBridge {
	/// No bridge: the state is read and written encrypted.
	#[default]
	None,
	/// Read a plaintext state too, write it encrypted: the apply that turns
	/// encryption on (`<Stack state_migrate=true>`) and the second rewrite of a
	/// passphrase rotation.
	Encrypt,
	/// Read an encrypted state too, write it plaintext: the first rewrite of
	/// a passphrase rotation, under the passphrase being retired. OpenTofu
	/// keys pbkdf2's salt by key-provider name, so two passphrases cannot
	/// both be `main` and a swap crosses plaintext instead.
	Decrypt,
}

impl StateEncryption {
	/// Encrypt state and plan files with a passphrase read from `env_var`.
	pub fn passphrase(env_var: impl Into<SmolStr>) -> Self {
		Self::Passphrase {
			env_var: env_var.into(),
			bridge: StateBridge::None,
		}
	}

	/// The encryption crossing `bridge`, see [`StateBridge`].
	pub fn with_bridge(self, bridge: StateBridge) -> Self {
		match self {
			Self::Passphrase { env_var, .. } => {
				Self::Passphrase { env_var, bridge }
			}
			Self::None => Self::None,
		}
	}

	/// The encryption reading its passphrase from `env_var` instead: how a
	/// rotation names the retiring value.
	pub fn with_env_var(self, env_var: impl Into<SmolStr>) -> Self {
		match self {
			Self::Passphrase { bridge, .. } => Self::Passphrase {
				env_var: env_var.into(),
				bridge,
			},
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
			Self::Passphrase { bridge, .. } => {
				let mut methods = value!({
					"aes_gcm": {
						"main": { "keys": "${key_provider.pbkdf2.main}" }
					}
				});
				let state = match bridge {
					StateBridge::None => {
						value!({ "method": "method.aes_gcm.main" })
					}
					StateBridge::Encrypt => value!({
						"method": "method.aes_gcm.main",
						"fallback": { "method": "method.unencrypted.migrate" }
					}),
					StateBridge::Decrypt => value!({
						"method": "method.unencrypted.migrate",
						"fallback": { "method": "method.aes_gcm.main" }
					}),
				};
				if *bridge != StateBridge::None {
					methods
						.insert("unencrypted", value!({ "migrate": {} }))
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

	/// The encrypting bridge reads a plaintext state through the
	/// `unencrypted` fallback and nothing else changes.
	#[beet_core::test]
	fn encrypt_bridge_adds_the_unencrypted_fallback() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.with_bridge(StateBridge::Encrypt)
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

	/// The decrypting bridge is the same pair with the roles swapped: the
	/// write is plaintext, the encrypted read is the fallback, and the plan
	/// stays encrypted.
	#[beet_core::test]
	fn decrypt_bridge_swaps_the_roles() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.with_env_var("TF_STATE_PASSPHRASE_OLD")
			.with_bridge(StateBridge::Decrypt)
			.to_json()
			.unwrap()
			.into_json();
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.unencrypted.migrate");
		json["state"]["fallback"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["plan"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		// the variable name is fixed; only the environment behind it moves
		json["key_provider"]["pbkdf2"]["main"]["passphrase"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.tf_state_passphrase}");
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
