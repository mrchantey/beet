//! `secrets/ls`: the keys of a vault.

use super::VaultParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// List a vault's keys (dotted paths for a tree vault) and its metadata,
/// never its values.
///
/// ```sh
/// beet secrets/ls                      # the entry's `.env.age`
/// beet secrets/ls --vault=mail-prod    # a declared vault
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("ls"),
	ParamsPartial = ParamsPartial::new::<VaultParams>()
)]
pub async fn SecretsLs(cx: ActionContext<Request>) -> Result<Response> {
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let doc = vault.read(&AgeIdentityFile::require()?).await?;
	let mut out = format!("{} [{}]\n", vault.describe(), vault.format);
	if let Some(meta) = doc.meta() {
		writeln!(out, "meta:")?;
		for (key, value) in meta.iter() {
			writeln!(out, "  {key} = {value}")?;
		}
	}
	let keys = doc.keys();
	writeln!(out, "{} key(s):", keys.len())?;
	for key in keys {
		writeln!(out, "  {key}")?;
	}
	Response::ok_text(out).xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn lists_keys_and_meta_without_values() {
		let mut world = VerbWorld::new();
		world.set("mail", "secrets.dkim.value", "PRIVATE").await;
		world
			.set("mail", "secrets.dkim.note", "the signing key")
			.await;
		world.set("mail", "meta.app", "mail").await;
		world
			.call_str(SecretsLs, Request::from_cli_str("--vault=mail"))
			.await
			.unwrap()
			.xnot()
			.xpect_contains("PRIVATE")
			.xpect_snapshot();
	}
}
