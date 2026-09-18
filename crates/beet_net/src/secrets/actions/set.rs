//! `secrets/set`: one record written, its group re-sealed.

use super::DocumentParams;
use super::name_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsSet`], surfaced in `--help`.
#[derive(Reflect)]
struct SetParams {
	/// The value. Absent and without `--from-env`, it is read from stdin: a
	/// prompt without echo on a terminal, else the piped input, since argv
	/// lands in shell history.
	value: Option<String>,
	/// Take the value from the process environment variable of the same
	/// name: the way out of `.env`, whose values the dotenv load already set.
	from_env: bool,
	/// The group to seal the record in, `default` when absent; created on
	/// first use with this identity file's own recipients.
	group: Option<String>,
	/// What consumes the record: `env_var` loads it into the process
	/// environment when the document loads; absent, it is kept and viewed.
	role: Option<String>,
	/// A plaintext note for the index, never a secret: what the value is
	/// for, where it is rotated.
	note: Option<String>,
}

/// Write one record to a document (created when it does not exist yet):
/// the value from `--value`, `--from-env` or stdin, into `--group` (default
/// `default`), with `--role` and `--note`, re-sealing the group to its
/// current recipient list. A record already in another group moves.
///
/// ```sh
/// beet secrets/set OPENAI_API_KEY --role=env_var       # prompts, no echo
/// beet secrets/set OPENAI_API_KEY --from-env --role=env_var --note="billing"
/// echo -n "$TOKEN" | beet secrets/set CF_API_TOKEN --group=agents
/// beet secrets/set dkim-example-com --document=mail-prod --value=..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("set/:name"),
	ParamsPartial = ParamsPartial::new::<(DocumentParams, SetParams)>()
)]
pub async fn SecretsSet(cx: ActionContext<Request>) -> Result<Response> {
	let name = name_param(&cx.input)?;
	let params = cx.input.parse_params::<SetParams>()?;
	let value = match (params.value, params.from_env) {
		(Some(_), true) => {
			bevybail!("pass `--value` or `--from-env`, not both")
		}
		(Some(value), false) => value,
		(None, true) => env_ext::var(&name)
			.map(|value| value.to_string())
			.map_err(|_| {
				bevyhow!(
					"`--from-env`: `{name}` is not set in the process \
					environment (the `.env` beside the entry loads into it)"
				)
			})?,
		(None, false) => read_stdin_value()?,
	};
	let record = SecretRecord {
		role: params.role.as_deref().map(str::parse).transpose()?,
		note: params.note.map(SmolStr::new),
		..default()
	}
	.with_group(
		params
			.group
			.as_deref()
			.unwrap_or(SecretRecord::DEFAULT_GROUP),
	);
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let mut document = handle.read_or_new().await?;
	document.set(&AgeIdentityFile::require()?, &name, &value, record)?;
	handle.write(&document).await?;
	let group = document.secrets[&name].group().to_string();
	Response::ok_text(format!(
		"set `{name}` in group `{group}` of {} ({} recipient(s))\n",
		handle.describe(),
		document.groups[group.as_str()].recipients.len()
	))
	.xok()
}

/// The value typed or piped in: one line without echo on a terminal, else
/// all of stdin with one trailing newline dropped.
fn read_stdin_value() -> Result<String> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			bevybail!("pass the value with `--value` or `--from-env`: this host has no stdin to read it from")
		} else {
			use std::io::IsTerminal;
			use std::io::Read;
			if std::io::stdin().is_terminal() {
				return terminal_ext::read_secret_line("value: ");
			}
			let mut value = String::new();
			std::io::stdin().read_to_string(&mut value)?;
			value
				.strip_suffix('\n')
				.map(|value| value.strip_suffix('\r').unwrap_or(value))
				.unwrap_or(&value)
				.to_string()
				.xok()
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
	use beet_core::prelude::*;

	/// The first `set` creates the document and `default`; a second with
	/// `--group` creates that group; a `set` of an existing record moves it.
	#[beet_core::test]
	async fn sets_creating_document_and_groups() {
		let mut fixture = VerbWorld::new();
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str(
					"--value=sk-test --role=env_var --note=billing",
				)
				.with_param("name", "OPENAI_API_KEY"),
			)
			.await
			.unwrap()
			.xpect_contains("set `OPENAI_API_KEY` in group `default`")
			.xpect_contains("1 recipient(s)");
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str("--value=cf-test --group=agents")
					.with_param("name", "CF_API_TOKEN"),
			)
			.await
			.unwrap()
			.xpect_contains("in group `agents`");
		let document = fixture.document().await;
		document.groups.len().xpect_eq(2);
		let opened = document.open(&fixture.identities()).unwrap();
		let key = opened.get("OPENAI_API_KEY").unwrap();
		key.value.as_str().xpect_eq("sk-test");
		key.record.role.xpect_eq(Some(SecretRole::EnvVar));
		key.record
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("billing");
		key.record.modified.xpect_some();
		// moved
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str("--value=sk-test --group=agents")
					.with_param("name", "OPENAI_API_KEY"),
			)
			.await
			.unwrap()
			.xpect_contains("in group `agents`");
		fixture
			.document()
			.await
			.open(&fixture.identities())
			.unwrap()
			.get("OPENAI_API_KEY")
			.unwrap()
			.group()
			.xpect_eq("agents");
		// a bad role names the good one
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--value=x --role=admin")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("env_var");
	}

	/// `--from-env` reads the process environment, the `.env` migration.
	/// Native only: the browser host has no mutable environment.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn sets_from_the_environment() {
		let mut fixture = VerbWorld::new();
		// SAFETY: test-only, a name no other test reads
		unsafe {
			env_ext::set_var("BEET_TEST_SET_FROM_ENV", "from-env").unwrap();
		}
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str("--from-env")
					.with_param("name", "BEET_TEST_SET_FROM_ENV"),
			)
			.await
			.unwrap()
			.xpect_contains("set `BEET_TEST_SET_FROM_ENV`");
		fixture
			.document()
			.await
			.open(&fixture.identities())
			.unwrap()
			.get("BEET_TEST_SET_FROM_ENV")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("from-env");
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--from-env")
					.with_param("name", "BEET_TEST_SET_UNSET"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("not set");
		unsafe {
			env_ext::remove_var("BEET_TEST_SET_FROM_ENV").unwrap();
		}
	}
}
