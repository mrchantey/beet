//! `secrets/get`: one value, printed.

use super::VaultParams;
use super::key_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Print one value of a vault. This is the deliberate print: the value is
/// the response, so `beet secrets/get OPENAI_API_KEY` pipes it wherever a
/// shell sends stdout. A string prints raw, anything else as json.
///
/// ```sh
/// beet secrets/get OPENAI_API_KEY
/// beet secrets/get secrets.dkim.value --vault=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("get/:key"),
	ParamsPartial = ParamsPartial::new::<VaultParams>()
)]
pub async fn SecretsGet(cx: ActionContext<Request>) -> Result<Response> {
	let key = key_param(&cx.input)?;
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let doc = vault.read(&AgeIdentityFile::require()?).await?;
	let value = doc
		.get(&key)
		.ok_or_else(|| bevyhow!("no `{key}` in vault {}", vault.describe()))?;
	match value {
		Value::Str(text) => text.to_string(),
		other => other.to_string_pretty()?,
	}
	.xmap(Response::ok_text)
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `set` then `get` on the flat default vault and on a dotted tree key.
	#[beet_core::test]
	async fn set_then_get_flat_and_dotted() {
		let mut world = VerbWorld::new();
		world.set(".env", "OPENAI_API_KEY", "sk-test").await;
		world.set("notes", "db.password", "hunter2").await;
		world
			.call_str(
				SecretsGet,
				Request::get("/").with_param("key", "OPENAI_API_KEY"),
			)
			.await
			.unwrap()
			.xpect_eq("sk-test");
		world
			.call_str(
				SecretsGet,
				Request::from_cli_str("--vault=notes")
					.with_param("key", "db.password"),
			)
			.await
			.unwrap()
			.xpect_eq("hunter2");
		world
			.call(
				SecretsGet,
				Request::from_cli_str("--vault=notes")
					.with_param("key", "nope"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no `nope`");
		world
			.call(SecretsGet, Request::get("/"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("a key is required");
	}
}
