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
/// beet secrets/get dkim-example-com --document=mail-prod
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
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let document = handle.read().await?;
	let opened = document.open(&AgeIdentityFile::require()?)?;
	match opened.get(&name) {
		Some(secret) => Response::ok_text(secret.value.to_string()).xok(),
		None => match document.group_of(&name) {
			Some(group) => bevybail!(
				"`{name}` is in group `{group}` of {}, which this identity \
				cannot open",
				handle.describe()
			),
			None => bevybail!("no record `{name}` in {}", handle.describe()),
		},
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
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
