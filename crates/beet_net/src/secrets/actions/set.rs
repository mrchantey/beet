//! `secrets/set`: one record written, its group re-sealed.

use super::DocumentParams;
use super::name_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsSet`], surfaced in `--help`.
#[derive(Reflect)]
struct SetParams {
	/// The value. Absent and without `--from-env` or `--generate`, it is
	/// read from stdin: a prompt without echo on a terminal, else the piped
	/// input, since argv lands in shell history.
	value: Option<String>,
	/// Take the value from the process environment variable of the same
	/// name: the way out of a developer's `.env`, whose values the dotenv
	/// load already set.
	from_env: bool,
	/// Mint the value in-process: an unambiguous alphanumeric string from
	/// the platform entropy source, the alphabet every generated credential
	/// uses, never printed. How a passphrase is born.
	generate: bool,
	/// The length of a generated value, 32 characters when absent.
	length: Option<usize>,
	/// The group to seal the record in, `default` when absent; created on
	/// first use with this identity file's own recipients.
	group: Option<String>,
	/// What consumes the record: `env_var` loads it into the process
	/// environment when the document loads; absent, it is kept and viewed.
	role: Option<String>,
	/// A plaintext note for the index, never a secret: one line of what the
	/// value is.
	note: Option<String>,
	/// How the value is rotated, one line: `manual:<where it is re-minted>`
	/// for a hand-made credential (`manual:dash.cloudflare.com/profile/api-tokens
	/// > Create Token > beet-deploy`), `remint` for one `--generate` mints,
	/// `replace:<resource>` for one an apply derives.
	rotation: Option<String>,
}

/// Write one record to a document (created when it does not exist yet):
/// the value from `--value`, `--from-env`, `--generate` (`--length` for
/// other than 32 characters) or stdin, into `--group` (default `default`),
/// with `--role`, `--note` and `--rotation`, re-sealing the group to its
/// current recipient list. A record already in another group moves.
///
/// ```sh
/// beet secrets/set OPENAI_API_KEY --role=env_var       # prompts, no echo
/// beet secrets/set OPENAI_API_KEY --from-env --role=env_var --note="openai api key" --rotation="manual:platform.openai.com/api-keys"
/// beet secrets/set TF_STATE_PASSPHRASE --generate --role=env_var
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
	let value = SecretsSet::value(&name, &params)?;
	let record = SecretRecord {
		role: params.role.as_deref().map(str::parse).transpose()?,
		note: params.note.map(SmolStr::new),
		rotation: params.rotation.as_deref().map(str::parse).transpose()?,
		..default()
	};
	let group = params
		.group
		.as_deref()
		.unwrap_or(SecretsDocument::DEFAULT_GROUP);
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let mut document = handle.read_or_new().await?;
	document.set(&AgeIdentityFile::require()?, group, &name, &value, record)?;
	handle.write(&document).await?;
	Response::ok_text(format!(
		"set `{name}` in group `{group}` of {} ({} recipient(s))\n",
		handle.describe(),
		document.groups[group].recipients.len()
	))
	.xok()
}

impl SecretsSet {
	/// The value the flags name: exactly one of `--value`, `--from-env` and
	/// `--generate`, else stdin.
	fn value(name: &str, params: &SetParams) -> Result<String> {
		let sources =
			[params.value.is_some(), params.from_env, params.generate]
				.into_iter()
				.filter(|given| *given)
				.count();
		if sources > 1 {
			bevybail!(
				"pass one of `--value`, `--from-env` and `--generate`, not \
				several"
			);
		}
		if let Some(value) = &params.value {
			return value.clone().xok();
		}
		if params.from_env {
			return env_ext::var(name).map(|value| value.to_string()).map_err(
				|_| {
					bevyhow!(
						"`--from-env`: `{name}` is not set in the process \
						environment"
					)
				},
			);
		}
		if params.generate {
			return Secret::generate(
				name,
				params.length.unwrap_or(Secret::GENERATED_LENGTH),
			)
			.map(|value| value.to_string());
		}
		read_stdin_value()
	}
}

/// The value typed or piped in: one line without echo on a terminal, else
/// all of stdin with one trailing newline dropped.
fn read_stdin_value() -> Result<String> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			bevybail!("pass the value with `--value`, `--from-env` or `--generate`: this host has no stdin to read it from")
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
	/// `--group` creates that group; a `set` of an existing record moves it;
	/// the rotation lands as typed.
	#[beet_core::test]
	async fn sets_creating_document_and_groups() {
		let mut fixture = VerbWorld::new();
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str(
					"--value=sk-test --role=env_var --note=billing \
					--rotation=manual:platform.openai.com/api-keys",
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
		key.record
			.rotation
			.clone()
			.xpect_eq(Some(SecretRotation::manual(
				"platform.openai.com/api-keys",
			)));
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
			.group
			.as_str()
			.xpect_eq("agents");
		// a bad role names the good one, a bad rotation the three forms
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
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--value=x --rotation=weekly")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("replace:<resource>");
	}

	/// `--generate` mints a value of the shared alphabet and never prints
	/// it; a length is honoured; two sources refuse.
	#[beet_core::test]
	async fn generates_a_value() {
		let mut fixture = VerbWorld::new();
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str("--generate --role=env_var")
					.with_param("name", "TF_STATE_PASSPHRASE"),
			)
			.await
			.unwrap()
			.xpect_contains("set `TF_STATE_PASSPHRASE`");
		let value = fixture
			.document()
			.await
			.open(&fixture.identities())
			.unwrap()
			.get("TF_STATE_PASSPHRASE")
			.unwrap()
			.value
			.clone();
		value.len().xpect_eq(Secret::GENERATED_LENGTH);
		value
			.chars()
			.all(|char| char.is_ascii_alphanumeric())
			.xpect_true();
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str("--generate --length=20")
					.with_param("name", "SHORT"),
			)
			.await
			.unwrap();
		fixture
			.document()
			.await
			.open(&fixture.identities())
			.unwrap()
			.get("SHORT")
			.unwrap()
			.value
			.len()
			.xpect_eq(20);
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--generate --length=8")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("too short");
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--generate --value=x")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("not several");
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
