use beet_core::prelude::*;

/// The tofu variable name the passphrase is threaded through in generated
/// config. Internal: never appears outside a `var.` reference and a `-var`
/// invocation, see [`StateEncryption::vars`].
pub const STATE_ENCRYPTION_VAR: &str = "tf_state_passphrase";

/// The same for the passphrase a roll is retiring, referenced only by the
/// first half of a crossing ([`StateCrossing::FromRetiring`]).
pub const STATE_ENCRYPTION_RETIRING_VAR: &str = "tf_state_passphrase_retiring";

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
		/// The half-step this render makes, `None` in the steady state.
		crossing: Option<StateCrossing>,
	},
}

/// A write that moves the state between encryption methods, and the reason
/// [`StateEncryption`] has more than one shape.
///
/// **OpenTofu keys pbkdf2's salt by the key provider's ADDRESS**, storing it in
/// the state envelope's `meta` as `key_provider.pbkdf2.<name>`. A provider
/// under a second name therefore finds no salt of its own and reads nothing,
/// which is why a passphrase cannot simply be swapped under one name and why
/// a roll takes the two halves below. Every one of them WRITES encrypted:
/// the backend never receives a plaintext state, which on a versioned bucket
/// would be kept rather than overwritten.
///
/// A crossing is never left in place, since a fallback that stays would also
/// read a state somebody replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateCrossing {
	/// Read a plaintext state too, write it encrypted: the one-off that turns
	/// encryption on for a stack that predates it. `unencrypted` needs no key
	/// material, so it is the one fallback with no salt to look up.
	FromPlaintext,
	/// Read under the primary key, holding the RETIRING passphrase this names,
	/// and write under the secondary: the first half of a roll, which is
	/// the only direction the retiring salt can be read from.
	FromRetiring(SmolStr),
	/// Read under the secondary key and write under the primary, both holding
	/// the current passphrase: the second half, which returns the salt to the
	/// name a steady-state read looks for.
	FromSecondary,
}

impl StateEncryption {
	/// Encrypt state and plan files with a passphrase read from `env_var`.
	pub fn passphrase(env_var: impl Into<SmolStr>) -> Self {
		Self::Passphrase {
			env_var: env_var.into(),
			crossing: None,
		}
	}

	/// The same encryption making `crossing`, see [`StateCrossing`].
	pub fn crossing(self, crossing: StateCrossing) -> Self {
		match self {
			Self::Passphrase { env_var, .. } => Self::Passphrase {
				env_var,
				crossing: Some(crossing),
			},
			Self::None => Self::None,
		}
	}

	/// The key provider a steady-state read and write both address, and the
	/// name the salt of every settled state is stored under.
	const PRIMARY: &'static str = "main";
	/// The key provider a roll parks the state under for one write, so the
	/// primary's name is free to take the new passphrase.
	const SECONDARY: &'static str = "next";

	/// The `terraform.encryption` block body, if enabled. `None` emits nothing,
	/// leaving the `terraform` block exactly as it is without this feature.
	///
	/// A target's `method` is a STATIC reference, which HCL's JSON syntax
	/// spells as the bare traversal (`"method.aes_gcm.main"`): wrapped in
	/// `${..}` it parses as a template expression and `tofu init` refuses it
	/// ("a single static variable reference is required"). The method's
	/// `keys` is an evaluated expression and keeps the interpolation.
	///
	/// One pbkdf2 key provider in the steady state, addressed
	/// [`PRIMARY`](Self::PRIMARY); a [`StateCrossing`] adds the second method a
	/// read may fall back to and chooses which key the write uses.
	pub fn to_json(&self) -> Option<Value> {
		let Self::Passphrase { crossing, .. } = self else {
			return None;
		};
		// per crossing: what the primary key holds, what the secondary holds
		// (absent in the steady state), which key a WRITE uses, and what a
		// READ may fall back to
		let (primary, secondary, write, fallback) = match crossing {
			None => (STATE_ENCRYPTION_VAR, None, Self::PRIMARY, None),
			Some(StateCrossing::FromPlaintext) => (
				STATE_ENCRYPTION_VAR,
				None,
				Self::PRIMARY,
				Some("method.unencrypted.migrate".to_string()),
			),
			// the retiring passphrase keeps the primary NAME, the only address
			// its salt is stored under, so the current one takes the secondary
			// for this one write
			Some(StateCrossing::FromRetiring(_)) => (
				STATE_ENCRYPTION_RETIRING_VAR,
				Some(STATE_ENCRYPTION_VAR),
				Self::SECONDARY,
				Some(Self::method(Self::PRIMARY)),
			),
			Some(StateCrossing::FromSecondary) => (
				STATE_ENCRYPTION_VAR,
				Some(STATE_ENCRYPTION_VAR),
				Self::PRIMARY,
				Some(Self::method(Self::SECONDARY)),
			),
		};
		let mut providers = Value::map();
		let mut aes_gcm = Value::map();
		let mut methods = Value::map();
		for (name, var) in
			[(Self::PRIMARY, Some(primary)), (Self::SECONDARY, secondary)]
				.into_iter()
				.filter_map(|(name, var)| var.map(|var| (name, var)))
		{
			providers
				.insert(
					name,
					value!({ "passphrase": (format!("${{var.{var}}}")) }),
				)
				.ok();
			aes_gcm
				.insert(
					name,
					value!({
						"keys": (format!("${{key_provider.pbkdf2.{name}}}"))
					}),
				)
				.ok();
		}
		if matches!(crossing, Some(StateCrossing::FromPlaintext)) {
			methods
				.insert("unencrypted", value!({ "migrate": {} }))
				.ok();
		}
		methods.insert("aes_gcm", aes_gcm).ok();
		let mut state = value!({ "method": (Self::method(write)) });
		if let Some(fallback) = fallback {
			state
				.insert("fallback", value!({ "method": fallback }))
				.ok();
		}
		Some(value!({
			"key_provider": { "pbkdf2": providers },
			"method": methods,
			"state": state,
			// a plan is written fresh, so it never crosses; it takes the write
			// key either way, which under `FromRetiring` is NOT the primary
			"plan": { "method": (Self::method(write)) },
		}))
	}

	/// The static reference naming one `aes_gcm` method, ie
	/// `method.aes_gcm.main`.
	fn method(key: &str) -> String { format!("method.aes_gcm.{key}") }

	/// The tofu variable names this encryption's rendered block references, so
	/// the config declares every one of them: an undeclared variable fails the
	/// init rather than the write. [`vars`](Self::vars) resolves their values.
	pub fn var_names(&self) -> Vec<&'static str> {
		match self {
			Self::None => Vec::new(),
			Self::Passphrase { crossing, .. } => {
				let mut names = vec![STATE_ENCRYPTION_VAR];
				if let Some(StateCrossing::FromRetiring(_)) = crossing {
					names.push(STATE_ENCRYPTION_RETIRING_VAR);
				}
				names
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
			Self::Passphrase { env_var, crossing } => {
				let read = |name: &str| {
					env_ext::var(name).map_err(|_| {
						bevyhow!(
							"state encryption is enabled but `{name}` is not set"
						)
					})
				};
				let mut vars =
					vec![(SmolStr::from(STATE_ENCRYPTION_VAR), read(env_var)?)];
				if let Some(StateCrossing::FromRetiring(retiring)) = crossing {
					vars.push((
						STATE_ENCRYPTION_RETIRING_VAR.into(),
						read(retiring)?,
					));
				}
				Ok(vars)
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

	/// Turning encryption on is the one crossing with a plaintext fallback,
	/// and `unencrypted` is the one fallback with no salt to look up.
	#[beet_core::test]
	fn from_plaintext_reads_plaintext_and_writes_encrypted() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.crossing(StateCrossing::FromPlaintext)
			.to_json()
			.unwrap()
			.into_json();
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["state"]["fallback"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.unencrypted.migrate");
		// one key provider: nothing names a second passphrase
		json["key_provider"]["pbkdf2"]
			.as_object()
			.unwrap()
			.keys()
			.collect::<Vec<_>>()
			.xpect_eq(vec!["main"]);
	}

	/// A roll's first half: the RETIRING passphrase keeps the primary
	/// NAME, since that is the only address its salt is stored under, and the
	/// current one takes the secondary for this one write. So the write lands
	/// under the current passphrase and the retiring one cannot read it.
	#[beet_core::test]
	fn from_retiring_writes_under_the_secondary_key() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.crossing(StateCrossing::FromRetiring("OLD".into()))
			.to_json()
			.unwrap()
			.into_json();
		json["key_provider"]["pbkdf2"]["main"]["passphrase"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.tf_state_passphrase_retiring}");
		json["key_provider"]["pbkdf2"]["next"]["passphrase"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.tf_state_passphrase}");
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.next");
		json["state"]["fallback"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		// the plan takes the WRITE key, not the primary: under this crossing
		// the primary holds the passphrase being retired, and a plan file
		// carries resource attributes in the clear
		json["plan"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.next");
		// nothing plaintext is reachable in either direction
		json["method"].get("unencrypted").xpect_none();
	}

	/// The second half returns the salt to the name a steady-state read looks
	/// for, both keys holding the current passphrase.
	#[beet_core::test]
	fn from_secondary_returns_the_salt_to_the_primary() {
		let json = StateEncryption::passphrase("TF_STATE_PASSPHRASE")
			.crossing(StateCrossing::FromSecondary)
			.to_json()
			.unwrap()
			.into_json();
		for key in ["main", "next"] {
			json["key_provider"]["pbkdf2"][key]["passphrase"]
				.as_str()
				.unwrap()
				.xpect_eq("${var.tf_state_passphrase}");
		}
		json["state"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.main");
		json["state"]["fallback"]["method"]
			.as_str()
			.unwrap()
			.xpect_eq("method.aes_gcm.next");
		json["method"].get("unencrypted").xpect_none();
	}

	/// Only the first half names a second passphrase, so only it declares the
	/// retiring variable: an undeclared one fails the init, and a declared one
	/// with nothing to supply it fails the plan.
	#[beet_core::test]
	fn only_the_retiring_half_declares_two_variables() {
		let names = |crossing: Option<StateCrossing>| {
			let encryption = StateEncryption::passphrase("PP");
			match crossing {
				Some(crossing) => encryption.crossing(crossing),
				None => encryption,
			}
			.var_names()
		};
		names(None).xpect_eq(vec![STATE_ENCRYPTION_VAR]);
		names(Some(StateCrossing::FromPlaintext))
			.xpect_eq(vec![STATE_ENCRYPTION_VAR]);
		names(Some(StateCrossing::FromSecondary))
			.xpect_eq(vec![STATE_ENCRYPTION_VAR]);
		names(Some(StateCrossing::FromRetiring("OLD".into()))).xpect_eq(vec![
			STATE_ENCRYPTION_VAR,
			STATE_ENCRYPTION_RETIRING_VAR,
		]);
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
