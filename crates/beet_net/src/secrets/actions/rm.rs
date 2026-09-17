//! `secrets/rm`: one key removed.

use super::VaultParams;
use super::key_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Remove one key from a vault and re-encrypt it to its declared recipients.
///
/// ```sh
/// beet secrets/rm OLD_TOKEN
/// beet secrets/rm secrets.retired --vault=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rm/:key"),
	ParamsPartial = ParamsPartial::new::<VaultParams>()
)]
pub async fn SecretsRm(cx: ActionContext<Request>) -> Result<Response> {
	let key = key_param(&cx.input)?;
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let identities = AgeIdentityFile::require()?;
	let mut doc = vault.read(&identities).await?;
	if !doc.remove(&key) {
		bevybail!("no `{key}` in vault {}", vault.describe());
	}
	let recipients = vault.write_recipients(&identities)?;
	vault.write(&doc, &recipients).await?;
	Response::ok_text(format!(
		"removed `{key}` from vault {}\n",
		vault.describe()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn removes_then_refuses_a_missing_key() {
		let mut world = VerbWorld::new();
		world.set("mail", "secrets.a.value", "1").await;
		world.set("mail", "secrets.b.value", "2").await;
		let request = || {
			Request::from_cli_str("--vault=mail")
				.with_param("key", "secrets.a.value")
		};
		world
			.call_str(SecretsRm, request())
			.await
			.unwrap()
			.xpect_contains("removed `secrets.a.value`");
		world
			.call(SecretsRm, request())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no `secrets.a.value`");
		world
			.call_str(SecretsLs, Request::from_cli_str("--vault=mail"))
			.await
			.unwrap()
			.xpect_contains("1 key(s)");
	}
}
