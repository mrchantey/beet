//! `secrets/rm`: one record removed, its group re-sealed.

use super::DocumentParams;
use super::name_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Remove one record from a document and re-seal its group. The old
/// ciphertext lives on in git history, so a removed secret its readers should
/// no longer hold is rotated at its source as well.
///
/// ```sh
/// beet secrets/rm OPENAI_API_KEY
/// beet secrets/rm dkim-example-com --vault=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("rm/:name"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsRm(cx: ActionContext<Request>) -> Result<Response> {
	let name = name_param(&cx.input)?;
	let vault = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let mut document = vault.read_document().await?;
	let record = document.remove(&AgeIdentityFile::require()?, &name)?;
	vault.write_document(&document).await?;
	Response::ok_text(format!(
		"removed `{name}` from group `{}` of {} ({} record(s) remain)\n",
		record.group(),
		vault.describe(),
		document.secrets.len()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn removes_a_record() {
		let mut fixture = VerbWorld::new();
		fixture.set("A", "1", default()).await;
		fixture.set("B", "2", default()).await;
		fixture
			.call_str(SecretsRm, Request::get("/").with_param("name", "A"))
			.await
			.unwrap()
			.xpect_contains("removed `A` from group `default`")
			.xpect_contains("1 record(s) remain");
		let document = fixture.document().await;
		document.secrets.contains_key("A").xpect_false();
		document
			.open(&fixture.identities())
			.unwrap()
			.get("B")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("2");
		fixture
			.call(SecretsRm, Request::get("/").with_param("name", "A"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no record `A`");
	}
}
