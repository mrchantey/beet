//! `secrets/rekey`: vaults re-encrypted to their current recipient lists.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Request params for [`SecretsRekey`], surfaced in `--help`.
#[derive(Reflect)]
struct RekeyParams {
	/// The vault to re-encrypt: a declared label, or the path or store uri
	/// of an undeclared file. Absent, every declared vault.
	vault: Option<String>,
	/// The recipients to encrypt to, comma separated `age1..,age1..`, for an
	/// undeclared vault (which otherwise takes this machine's own identity)
	/// or to override a declared list.
	recipients: Option<String>,
}

/// Re-encrypt one vault (`--vault`) or every declared vault to its current
/// recipient list, the second half of adding a human (their recipient into
/// the declarations first) and the first half of removing one. A declared
/// vault with an empty list is refused; an undeclared one takes
/// `--recipients`.
///
/// ```sh
/// beet secrets/rekey                                     # every declared vault
/// beet secrets/rekey --vault=mail-prod
/// beet secrets/rekey --vault=~/p.toml.age --recipients=age1..,age1..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rekey"),
	ParamsPartial = ParamsPartial::new::<RekeyParams>()
)]
pub async fn SecretsRekey(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<RekeyParams>()?;
	let identities = AgeIdentityFile::require()?;
	let overrides = params
		.recipients
		.as_deref()
		.map(parse_recipients)
		.transpose()?;
	let vaults = match params.vault.as_deref() {
		Some(selector) => {
			vec![VaultHandle::resolve(&cx.caller, Some(selector)).await?]
		}
		None => VaultHandle::declared(&cx.caller)
			.await?
			.into_iter()
			.map(|(_, vault)| vault)
			.collect::<Result<Vec<_>>>()?,
	};
	if vaults.is_empty() {
		bevybail!(
			"no vault is declared: name one with `--vault=<label or path>`"
		);
	}
	let mut out = String::new();
	for vault in vaults {
		if !vault.exists().await? {
			writeln!(out, "skipped {}: not written yet", vault.describe())?;
			continue;
		}
		let recipients = match &overrides {
			Some(recipients) => recipients.clone(),
			None => vault.write_recipients(&identities)?,
		};
		let doc = vault.read(&identities).await?;
		vault.write(&doc, &recipients).await?;
		writeln!(
			out,
			"rekeyed {} to {} recipient(s)",
			vault.describe(),
			recipients.len()
		)?;
	}
	Response::ok_text(out).xok()
}

/// A comma separated recipient list, each validated.
fn parse_recipients(list: &str) -> Result<Vec<AgeRecipient>> {
	let recipients = list
		.split(',')
		.map(str::trim)
		.filter(|item| !item.is_empty())
		.map(AgeRecipient::new)
		.collect::<Result<Vec<_>>>()?;
	if recipients.is_empty() {
		bevybail!("`--recipients` names no recipient");
	}
	recipients.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A rekey to a second recipient lets a second identity read the vault.
	#[beet_core::test]
	async fn rekeys_to_a_second_recipient() {
		let mut world = VerbWorld::new();
		world.set("mail", "a", "1").await;
		let other = AgeIdentity::generate();
		let mut others = AgeIdentityFile::default();
		others.push(other.clone());
		let vault =
			VaultHandle::new(world.store.clone(), "secrets/mail.toml.age")
				.unwrap();
		vault.read(&others).await.unwrap_err();
		world
			.call_str(
				SecretsRekey,
				Request::from_cli_str(&format!(
					"--vault=mail --recipients={},{}",
					world.identity.to_recipient(),
					other.to_recipient()
				)),
			)
			.await
			.unwrap()
			.xpect_contains("to 2 recipient(s)");
		vault
			.read(&others)
			.await
			.unwrap()
			.get("a")
			.xpect_eq(Some(Value::str("1")));
		// every declared vault: `mail` again, `notes` skipped as unwritten
		world
			.call_str(SecretsRekey, Request::get("/"))
			.await
			.unwrap()
			.xpect_contains("rekeyed `mail`")
			.xpect_contains("skipped `notes`");
		// and back to the declared list alone locks the other out
		vault.read(&others).await.unwrap_err();
	}
}
