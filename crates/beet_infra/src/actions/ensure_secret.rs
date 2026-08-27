//! The deploy step that mints a stack's secrets before the apply consumes them.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<EnsureSecret secret="db-password" variable="db_password"/>` — create the
/// named [`SecretRef`] in parameter store if it is not already there, with a
/// generated value, and hand that value to the apply that needs it.
///
/// Create-IF-MISSING, never rotate. A secret this action minted is already the
/// master password of a running database and the credential of a running mail
/// server; regenerating it on the next deploy would lock the stack out of its
/// own resources. So the parameter is the source of truth from the moment it
/// exists, and every later deploy reads it back.
///
/// The generated value is ALPHANUMERIC and nothing else. These passwords land
/// in shell environment files, in connection strings and in a JSON config
/// spliced together on a booting box: restraint in the alphabet here is far
/// cheaper than correct escaping in every one of those places, and it costs
/// only length to make up the entropy.
///
/// Nothing here logs the value, and the parameter is a `SecureString`, so the
/// only place it exists in the clear is the tofu state that the apply writes
/// (see [`StateEncryption`], which is not optional for a stack that runs this).
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(EnsureSecretAction)]
pub struct EnsureSecret {
	/// Which secret, named the way the block that reads it names it.
	secret: SecretRef,
	/// The tofu variable to hand the value to, when the resource that consumes
	/// it is created by the same apply (an RDS master password). Absent for a
	/// secret only a running process reads (the mail box's admin password),
	/// which terraform never needs to see at all.
	#[set_with(unwrap_option, into)]
	variable: Option<SmolStr>,
	/// Generated length, in characters of [`ALPHABET`](Self::ALPHABET).
	length: usize,
}

impl Default for EnsureSecret {
	fn default() -> Self { Self::new(SecretRef::default()) }
}

impl EnsureSecret {
	/// Unambiguous alphanumerics: no `0`/`O` or `1`/`l`, since these values are
	/// read aloud and typed by hand during an incident.
	pub const ALPHABET: &'static [u8] =
		b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

	/// 32 characters of [`ALPHABET`](Self::ALPHABET) is ~185 bits, which is
	/// more than the 128 anything here needs and still fits on one line.
	pub const LENGTH: usize = 32;

	pub fn new(secret: SecretRef) -> Self {
		Self {
			secret,
			variable: None,
			length: Self::LENGTH,
		}
	}

	/// A generated value, drawn from the platform entropy source.
	pub fn generate(&self) -> Result<SmolStr> {
		if self.length < 16 {
			bevybail!(
				"secret '{}' is {} characters: too short to be worth generating",
				self.secret.label(),
				self.length
			);
		}
		let mut source = RandomSource::default();
		(0..self.length)
			.map(|_| {
				Self::ALPHABET[source.random_range(0..Self::ALPHABET.len())]
					as char
			})
			.collect::<String>()
			.xmap(SmolStr::from)
			.xok()
	}
}

/// Reads the parameter, mints it if it is not there, and passes the value on to
/// the apply as a `-var` when one was named.
///
/// Idempotent by construction, and cheap when there is nothing to do: an
/// existing parameter is one read and no write.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EnsureSecretAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let ensure = cx.caller.get_cloned::<EnsureSecret>().await?;
	let stack = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| query.resolve(entity))
		.await?;
	let region = stack.region().clone();
	let name = ensure.secret().name(&stack);

	let value = match ssm_ext::get(&region, &name).await? {
		Some(value) => {
			info!("secret {name} already exists");
			value
		}
		None => {
			// the loser of a race re-reads rather than overwriting, so two
			// deploys can never disagree about which value is in use.
			let generated = ensure.generate()?;
			match ssm_ext::create(&region, &name, &generated).await {
				Ok(()) => {
					info!("minted secret {name}");
					generated.to_string()
				}
				Err(err) if ssm_ext::is_already_exists(&err) => {
					info!("secret {name} was minted concurrently, re-reading");
					ssm_ext::get(&region, &name).await?.ok_or_else(|| {
						bevyhow!("secret {name} vanished between writes")
					})?
				}
				Err(err) => return Err(err),
			}
		}
	};

	let input = match ensure.variable() {
		Some(key) => cx.input.with_param(key, &value),
		None => cx.input,
	};
	Pass(input).xok()
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The alphabet is the whole reason this action generates rather than
	/// taking `openssl rand -base64`: a `/` or a `+` in a password that gets
	/// spliced into a DSN or a shell env file is a bug in a different file.
	#[beet_core::test]
	fn generates_alphanumerics_only() {
		let value = EnsureSecret::new(SecretRef::new("db-password"))
			.generate()
			.unwrap();
		value.len().xpect_eq(EnsureSecret::LENGTH);
		value
			.chars()
			.all(|char| char.is_ascii_alphanumeric())
			.xpect_true();
	}

	/// Two calls must not agree, or the entropy source is not one.
	#[beet_core::test]
	fn generates_a_different_value_each_time() {
		let ensure = EnsureSecret::new(SecretRef::new("db-password"));
		(ensure.generate().unwrap() != ensure.generate().unwrap()).xpect_true();
	}

	/// A length that would not survive being guessed is a config-time error,
	/// not a weak password nobody notices.
	#[beet_core::test]
	fn rejects_a_length_that_is_not_worth_generating() {
		EnsureSecret::new(SecretRef::new("db-password"))
			.with_length(8)
			.generate()
			.unwrap_err()
			.to_string()
			.xpect_contains("too short");
	}

	/// The variable is opt-in: the mail box's admin password is read by the box
	/// at boot and never by terraform, so declaring it as a `-var` would put a
	/// secret in the plan output for nothing.
	#[beet_core::test]
	fn the_tofu_variable_is_opt_in() {
		EnsureSecret::new(SecretRef::new("mail-admin-password"))
			.variable()
			.is_none()
			.xpect_true();
		EnsureSecret::new(SecretRef::new("db-password"))
			.with_variable("db_password")
			.variable()
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("db_password");
	}

	/// The name the action reads is the one the database's boot script reads,
	/// which is the whole reason `SecretRef` exists.
	#[beet_core::test]
	fn the_name_is_the_database_ref_composition() {
		let stack = Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		let database = DatabaseRef::new("db");
		EnsureSecret::new(database.secret())
			.secret()
			.name(&stack)
			.xpect_eq(database.secret_name(&stack));
	}
}
