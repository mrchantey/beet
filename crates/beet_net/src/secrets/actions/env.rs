//! `secrets/env`: an env vault as shell exports.

use super::VaultParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Print an env vault as `export KEY='value'` lines for `eval`. This is the
/// deliberate print: the values are the response. A tree vault is refused,
/// only the `.env` grammar becomes an environment.
///
/// ```sh
/// eval "$(beet secrets/env)"
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("env"),
	ParamsPartial = ParamsPartial::new::<VaultParams>()
)]
pub async fn SecretsEnv(cx: ActionContext<Request>) -> Result<Response> {
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let doc = vault.read(&AgeIdentityFile::require()?).await?;
	let mut out = String::new();
	for (key, value) in doc.as_env()?.pairs() {
		writeln!(out, "export {key}='{}'", value.replace('\'', "'\\''"))?;
	}
	Response::ok_text(out).xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn exports_an_env_vault_and_refuses_a_tree() {
		let mut world = VerbWorld::new();
		world.set(".env", "TOKEN", "it's").await;
		world
			.call_str(SecretsEnv, Request::get("/"))
			.await
			.unwrap()
			.xpect_eq("export TOKEN='it'\\''s'\n");
		world.set("mail", "a", "1").await;
		world
			.call(SecretsEnv, Request::from_cli_str("--vault=mail"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("tree vault");
	}
}
