//! `secrets/rekey`: a document's groups re-sealed to their current lists.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Re-seal every group this identity can open to its current recipient
/// list, naming the ones it cannot: the second half of adding a reader,
/// after their recipient is added to a group's list in the document. With
/// no `--document` every declared document is rekeyed (the conventional
/// `secrets.toml` when none is declared). Removing a reader is a rekey plus
/// rotating what they could read, since git history keeps the old
/// ciphertext; `revoke` does both. An age file is re-encrypted by
/// `vault/rekey` instead.
///
/// ```sh
/// beet secrets/rekey                          # every declared document
/// beet secrets/rekey --document=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rekey"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsRekey(cx: ActionContext<Request>) -> Result<Response> {
	let selector = cx.input.parse_params::<DocumentParams>()?.document;
	let identities = AgeIdentityFile::require()?;
	let mut handles = match &selector {
		Some(_) => Vec::new(),
		None => SecretsHandle::declared(&cx.caller)
			.await?
			.into_iter()
			.map(|(_, handle)| handle)
			.collect::<Result<Vec<_>>>()?,
	};
	// named, or nothing declared: the one document the selector resolves
	if handles.is_empty() {
		handles.push(
			SecretsHandle::resolve(&cx.caller, selector.as_deref()).await?,
		);
	}
	let mut out = String::new();
	for handle in handles {
		let mut document = handle.read().await?;
		let report = document.rekey(&identities)?;
		handle.write(&document).await?;
		writeln!(
			out,
			"rekeyed {}: {} group(s) re-sealed{}{}",
			handle.describe(),
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
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
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
			.set(&strangers, "theirs", "B", "2", default())
			.unwrap();
		let handle = fixture.secrets("secrets.toml");
		handle.write(&document).await.unwrap();
		document.open(&alice_file).unwrap().get("A").xpect_none();
		fixture
			.call_str(SecretsRekey, Request::get("/"))
			.await
			.unwrap()
			.xpect_contains("1 group(s) re-sealed (default)")
			.xpect_contains("not a member of theirs");
		handle
			.read()
			.await
			.unwrap()
			.open(&alice_file)
			.unwrap()
			.get("A")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("1");
	}
}
