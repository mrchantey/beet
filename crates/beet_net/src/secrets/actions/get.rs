//! `secrets/get`: one value, printed.

use super::DocumentParams;
use super::name_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Print one record's value. This is the deliberate print: the value is the
/// response, so `beet secrets/get OPENAI_API_KEY` pipes it wherever a shell
/// sends stdout. Mind what is recording your session.
///
/// ```sh
/// beet secrets/get OPENAI_API_KEY
/// beet secrets/get dkim-example-com --vault=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("get/:name"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsGet(cx: ActionContext<Request>) -> Result<Response> {
	let name = name_param(&cx.input)?;
	let vault = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let document = vault.read_document().await?;
	let opened = document.open(&AgeIdentityFile::require()?)?;
	match opened.get(&name) {
		Some(secret) => Response::ok_text(secret.value.to_string()).xok(),
		None => match document.secrets.get(&name) {
			Some(record) => bevybail!(
				"`{name}` is in group `{}` of {}, which this identity cannot \
				open",
				record.group(),
				vault.describe()
			),
			None => bevybail!("no record `{name}` in {}", vault.describe()),
		},
	}
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn prints_a_value_and_names_a_missing_one() {
		let mut fixture = VerbWorld::new();
		fixture.set("OPENAI_API_KEY", "sk-test", default()).await;
		fixture
			.call_str(
				SecretsGet,
				Request::get("/").with_param("name", "OPENAI_API_KEY"),
			)
			.await
			.unwrap()
			.xpect_eq("sk-test");
		fixture
			.call(SecretsGet, Request::get("/").with_param("name", "NOPE"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no record `NOPE`");
		fixture
			.call(SecretsGet, Request::get("/"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("secrets/get OPENAI_API_KEY");
	}
}
