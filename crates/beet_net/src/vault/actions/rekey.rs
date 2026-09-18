//! `vault/rekey`: an age file re-encrypted to a new recipient list.

use super::VaultParams;
use super::write_recipients;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`VaultRekey`], surfaced in `--help`.
#[derive(Reflect)]
struct RekeyParams {
	/// Who may read the file from now on, comma separated `age1..,age1..`;
	/// absent, this identity file's own recipients.
	recipients: Option<String>,
}

/// Re-encrypt an age file to a recipient list: the second half of adding a
/// reader. Removing one is a rekey plus rotating what they could read, since
/// git history keeps the old ciphertext. A secrets document's groups are
/// re-sealed by `secrets/rekey` instead.
///
/// ```sh
/// beet vault/rekey --vault=infra/cert.pem.age --recipients=age1..,age1..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rekey"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, RekeyParams)>()
)]
pub async fn VaultRekey(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<RekeyParams>()?;
	let vault = VaultParams::resolve(&cx.input)?;
	let identities = AgeIdentityFile::require()?;
	let recipients =
		write_recipients(params.recipients.as_deref(), &identities)?;
	let plaintext = vault.read(&identities).await?;
	vault.write(&plaintext, &recipients).await?;
	Response::ok_text(format!(
		"rekeyed {} to {} recipient(s)\n",
		vault.describe(),
		recipients.len()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A rekey to a second recipient lets a second identity read the file,
	/// and a rekey back to this identity alone locks it out again.
	#[beet_core::test]
	async fn rekeys_to_a_second_recipient() {
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
				VaultRekey,
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
				VaultRekey,
				Request::from_cli_str(&format!("--vault={uri}")),
			)
			.await
			.unwrap()
			.xpect_contains("to 1 recipient(s)");
		vault.read(&others).await.unwrap_err();
	}
}
