//! `secrets/rekey`: a document's groups, or an age file, re-sealed to their
//! current recipients.

use super::DocumentParams;
use super::write_recipients;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Request params for [`SecretsRekey`], surfaced in `--help`.
#[derive(Reflect)]
struct RekeyParams {
	/// For an age file only: who may read it from now on, comma separated
	/// `age1..,age1..`; absent, this identity file's own recipients. A
	/// document's lists live in the document.
	recipients: Option<String>,
}

/// Re-seal every group this identity can open to its current recipient
/// list, naming the ones it cannot: the second half of adding a reader,
/// after their recipient is added to a group's list in the document. With
/// no `--vault` every declared document is rekeyed (the conventional
/// `secrets.toml.age` when none is declared). Given the path of an age file
/// rather than a document, the file is re-encrypted to `--recipients`.
/// Removing a reader is a rekey plus rotating what they could read, since
/// git history keeps the old ciphertext; `revoke` does both.
///
/// ```sh
/// beet secrets/rekey                          # every declared document
/// beet secrets/rekey --vault=mail-prod
/// beet secrets/rekey --vault=infra/cert.pem.age --recipients=age1..,age1..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rekey"),
	ParamsPartial = ParamsPartial::new::<(DocumentParams, RekeyParams)>()
)]
pub async fn SecretsRekey(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<RekeyParams>()?;
	let selector = cx.input.parse_params::<DocumentParams>()?.vault;
	let identities = AgeIdentityFile::require()?;
	let mut handles = match &selector {
		Some(_) => Vec::new(),
		None => VaultHandle::declared(&cx.caller)
			.await?
			.into_iter()
			.map(|(_, handle)| handle)
			.collect::<Result<Vec<_>>>()?,
	};
	// named, or nothing declared: the one document the selector resolves
	if handles.is_empty() {
		handles.push(
			VaultHandle::resolve_document(&cx.caller, selector.as_deref())
				.await?,
		);
	}
	let mut out = String::new();
	for vault in handles {
		let bytes = vault.read_bytes().await?;
		if VaultHandle::is_age_file(&bytes) {
			let recipients =
				write_recipients(params.recipients.as_deref(), &identities)?;
			let plaintext = identities.decrypt(&bytes)?;
			vault.write(&plaintext, &recipients).await?;
			writeln!(
				out,
				"rekeyed {} to {} recipient(s)",
				vault.describe(),
				recipients.len()
			)?;
			continue;
		}
		if params.recipients.is_some() {
			bevybail!(
				"{} is a document: its recipients are edited in its group \
				lists, not with `--recipients`",
				vault.describe()
			);
		}
		let mut document = vault.read_document().await?;
		let report = document.rekey(&identities)?;
		vault.write_document(&document).await?;
		writeln!(
			out,
			"rekeyed {}: {} group(s) re-sealed{}{}",
			vault.describe(),
			report.rekeyed.len(),
			match report.rekeyed.is_empty() {
				true => String::new(),
				false => format!(" ({})", report.rekeyed.join(", ")),
			},
			match report.locked.is_empty() {
				true => String::new(),
				false => format!(
					"; not a member of {}, left as they were",
					report.locked.join(", ")
				),
			}
		)?;
	}
	Response::ok_text(out).xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A recipient added to a group's list reads it after `rekey`, and a
	/// group this identity is not in is left as it was.
	#[beet_core::test]
	async fn rekeys_a_document() {
		let mut fixture = VerbWorld::new();
		fixture.set("A", "1", default()).await;
		let alice = AgeIdentity::generate();
		let mut alice_file = AgeIdentityFile::default();
		alice_file.push(alice.clone());
		let stranger = AgeIdentity::generate();
		let mut strangers = AgeIdentityFile::default();
		strangers.push(stranger.clone());
		let mut document = fixture.document().await;
		document
			.groups
			.get_mut("default")
			.unwrap()
			.recipients
			.push(alice.to_recipient());
		document.groups.insert(
			"theirs".into(),
			SecretsGroup::new(vec![stranger.to_recipient()]),
		);
		document
			.set(
				&strangers,
				"B",
				"2",
				SecretRecord::default().with_group("theirs"),
			)
			.unwrap();
		let vault = fixture.vault("secrets.toml.age");
		vault.write_document(&document).await.unwrap();
		document.open(&alice_file).unwrap().get("A").xpect_none();
		fixture
			.call_str(SecretsRekey, Request::get("/"))
			.await
			.unwrap()
			.xpect_contains("1 group(s) re-sealed (default)")
			.xpect_contains("not a member of theirs");
		vault
			.read_document()
			.await
			.unwrap()
			.open(&alice_file)
			.unwrap()
			.get("A")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("1");
		// `--recipients` is for age files
		fixture
			.call(SecretsRekey, Request::from_cli_str("--recipients=age1x"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("group lists");
	}

	/// The file form: a rekey to a second recipient lets a second identity
	/// read the file, and a rekey back to this identity alone locks it out.
	#[beet_core::test]
	async fn rekeys_an_age_file() {
		let mut fixture = VerbWorld::new();
		fixture.write("cert.pem.age", "a = 1\n").await;
		let other = AgeIdentity::generate();
		let mut others = AgeIdentityFile::default();
		others.push(other.clone());
		let vault = fixture.vault("cert.pem.age");
		let uri = fixture.uri("cert.pem.age");
		vault.read(&others).await.unwrap_err();
		fixture
			.call_str(
				SecretsRekey,
				Request::from_cli_str(&format!(
					"--vault={uri} --recipients={},{}",
					fixture.identity.to_recipient(),
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
			.xpect_eq(b"a = 1\n".to_vec());
		fixture
			.call_str(
				SecretsRekey,
				Request::from_cli_str(&format!("--vault={uri}")),
			)
			.await
			.unwrap()
			.xpect_contains("to 1 recipient(s)");
		vault.read(&others).await.unwrap_err();
	}
}
