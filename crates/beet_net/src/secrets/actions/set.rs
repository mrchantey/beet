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
	/// Take the value of another record of the document, never printed: how
	/// a passphrase about to be rolled is kept beside its replacement
	/// (`secrets/set TF_STATE_PASSPHRASE_OLD --copy=TF_STATE_PASSPHRASE`).
	copy: Option<String>,
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
	/// How the value is rolled: `manual:<where it is re-minted>` for a
	/// hand-made credential, the full url first and one dashboard step or
	/// permission per line (`manual:https://dash.cloudflare.com/profile/api-tokens
	/// > Create Token > beet-deploy`), `remint` for one `--generate` mints,
	/// `replace:<resource>` for one an apply derives.
	roll: Option<String>,
	/// When the value stops authenticating, for a credential issued with a
	/// lifetime: a day (`2027-01-05`, midnight UTC) or an ISO 8601 instant.
	/// Every launch that loads it warns in the fortnight before, and
	/// `secrets/check` fails after. Re-setting a record restates it, as it
	/// restates the note and roll.
	expires: Option<String>,
	// no flag for the record's metadata: it describes the value, so a re-set
	// of the same value keeps it and a new value drops it
}

/// Write one record to a document (created when it does not exist yet):
/// the value from `--value`, `--from-env`, `--copy` (another record's),
/// `--generate` (`--length` for other than 32 characters) or stdin, into
/// `--group` (default `default`), with `--role`, `--note`, `--roll` and
/// `--expires`, re-sealing the group to its current recipient list. A record
/// already in another group moves.
///
/// ```sh
/// beet secrets/set OPENAI_API_KEY --role=env_var       # prompts, no echo
/// beet secrets/set OPENAI_API_KEY --from-env --role=env_var --note="openai api key" --roll="manual:platform.openai.com/api-keys"
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
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let mut document = handle.read_or_new().await?;
	let identity = AgeIdentityFile::require()?;
	let value = SecretsSet::value(&name, &params, &document, &identity)?;
	let metadata = document
		.open(&identity)
		.ok()
		.and_then(|opened| opened.get(&name).cloned())
		.filter(|held| held.value == value)
		.map(|held| held.record.metadata)
		.unwrap_or_default();
	let record = SecretRecord {
		role: params.role.as_deref().map(str::parse).transpose()?,
		note: params.note.map(SmolStr::new),
		roll: params.roll.as_deref().map(str::parse).transpose()?,
		expires: params
			.expires
			.as_deref()
			.map(SecretsSet::expires)
			.transpose()?,
		metadata,
		..default()
	};
	let group = params
		.group
		.as_deref()
		.unwrap_or(SecretsDocument::DEFAULT_GROUP);
	document.set(&identity, group, &name, &value, record)?;
	handle.write(&document).await?;
	Response::ok_text(format!(
		"set `{name}` in group `{group}` of {} ({} recipient(s))\n",
		handle.describe(),
		document.groups[group].recipients.len()
	))
	.xok()
}

impl SecretsSet {
	/// `--expires`: a day, read as its midnight UTC, or a full instant.
	fn expires(text: &str) -> Result<Timestamp> {
		Timestamp::parse_rfc3339(text)
			.or_else(|| Date::parse(text).ok().map(|day| day.timestamp()))
			.ok_or_else(|| {
				bevyhow!(
					"`--expires={text}` is neither a day (`2027-01-05`) nor an \
					ISO 8601 instant (`2027-01-05T00:00:00Z`)"
				)
			})
	}

	/// The value the flags name: exactly one of `--value`, `--from-env`,
	/// `--copy` and `--generate`, else stdin.
	fn value(
		name: &str,
		params: &SetParams,
		document: &SecretsDocument,
		identity: &AgeIdentityFile,
	) -> Result<String> {
		let sources = [
			params.value.is_some(),
			params.from_env,
			params.copy.is_some(),
			params.generate,
		]
		.into_iter()
		.filter(|given| *given)
		.count();
		if sources > 1 {
			bevybail!(
				"pass one of `--value`, `--from-env`, `--copy` and \
				`--generate`, not several"
			);
		}
		if let Some(value) = &params.value {
			return value.clone().xok();
		}
		if let Some(source) = &params.copy {
			return document
				.open(identity)?
				.get(source)
				.map(|secret| secret.value.to_string())
				.ok_or_else(|| {
					bevyhow!(
						"`--copy`: no record `{source}` this identity can open \
						in the document"
					)
				});
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
	/// the roll lands as typed and a day's expiry as its midnight.
	#[beet_core::test]
	async fn sets_creating_document_and_groups() {
		let mut fixture = VerbWorld::new();
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str(
					"--value=sk-test --role=env_var --note=billing \
					--roll=manual:platform.openai.com/api-keys \
					--expires=2027-01-05",
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
			.roll
			.clone()
			.xpect_eq(Some(SecretRoll::manual("platform.openai.com/api-keys")));
		key.record.modified.xpect_some();
		key.record
			.expires
			.xpect_eq(Some(Date::parse("2027-01-05").unwrap().timestamp()));
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
		// a bad role names the good one, a bad roll the three forms
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
				Request::from_cli_str("--value=x --roll=weekly")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("replace:<resource>");
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--value=x --expires=soon")
					.with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`2027-01-05`");
	}

	/// A record's metadata describes its value: a re-set of the same value
	/// (a new note through `--from-env`) keeps it, a new value drops it.
	#[beet_core::test]
	async fn keeps_metadata_only_for_the_same_value() {
		let mut fixture = VerbWorld::new();
		fixture
			.set("CLOUDFLARE_API_TOKEN", "token-a", SecretRecord {
				metadata: BTreeMap::from([(
					"grants".into(),
					"DNS Write".into(),
				)]),
				..default()
			})
			.await;
		let metadata = async |fixture: &mut VerbWorld| {
			fixture
				.document()
				.await
				.open(&fixture.identities())
				.unwrap()
				.get("CLOUDFLARE_API_TOKEN")
				.unwrap()
				.record
				.metadata
				.clone()
		};
		let set = async |fixture: &mut VerbWorld, args: &str| {
			fixture
				.call_str(
					SecretsSet,
					Request::from_cli_str(args)
						.with_param("name", "CLOUDFLARE_API_TOKEN"),
				)
				.await
				.unwrap();
		};
		set(&mut fixture, "--value=token-a --note=renamed").await;
		metadata(&mut fixture)
			.await
			.get("grants")
			.cloned()
			.xpect_eq(Some(SmolStr::from("DNS Write")));
		set(&mut fixture, "--value=token-b").await;
		metadata(&mut fixture).await.is_empty().xpect_true();
	}

	/// `--copy` takes another record's value without printing it, and names
	/// a source it cannot read.
	#[beet_core::test]
	async fn copies_another_record() {
		let mut fixture = VerbWorld::new();
		fixture
			.set("TF_STATE_PASSPHRASE", "current", default())
			.await;
		fixture
			.call_str(
				SecretsSet,
				Request::from_cli_str(
					"--copy=TF_STATE_PASSPHRASE --role=env_var",
				)
				.with_param("name", "TF_STATE_PASSPHRASE_OLD"),
			)
			.await
			.unwrap()
			.xpect_contains("set `TF_STATE_PASSPHRASE_OLD`")
			.xnot()
			.xpect_contains("current");
		fixture
			.document()
			.await
			.open(&fixture.identities())
			.unwrap()
			.get("TF_STATE_PASSPHRASE_OLD")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("current");
		fixture
			.call(
				SecretsSet,
				Request::from_cli_str("--copy=NOPE").with_param("name", "X"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no record `NOPE`");
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
