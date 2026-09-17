//! `secrets/set`: one key written, the vault re-encrypted.

use super::VaultParams;
use super::key_param;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsSet`], surfaced in `--help`.
#[derive(Reflect)]
struct SetParams {
	/// The value. Absent, it is read from stdin: a prompt without echo on a
	/// terminal, else the piped input, since argv lands in shell history.
	value: Option<String>,
}

/// Write one key to a vault (creating the vault when it does not exist yet)
/// and re-encrypt it to its declared recipients; an undeclared vault is
/// encrypted to this machine's own identity and says so. A tree vault takes
/// a dotted path.
///
/// ```sh
/// beet secrets/set OPENAI_API_KEY                        # prompts, no echo
/// echo -n "$TOKEN" | beet secrets/set CF_API_TOKEN       # piped
/// beet secrets/set secrets.dkim.value --vault=mail-prod --value=..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("set/:key"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, SetParams)>()
)]
pub async fn SecretsSet(cx: ActionContext<Request>) -> Result<Response> {
	let key = key_param(&cx.input)?;
	let value = match cx.input.parse_params::<SetParams>()?.value {
		Some(value) => value,
		None => read_stdin_value()?,
	};
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let identities = AgeIdentityFile::require()?;
	let mut doc = vault.read_or_empty(&identities).await?;
	doc.set(&key, value)?;
	let recipients = vault.write_recipients(&identities)?;
	vault.write(&doc, &recipients).await?;
	Response::ok_text(format!(
		"set `{key}` in vault {} ({} recipients)\n",
		vault.describe(),
		recipients.len()
	))
	.xok()
}

/// The value typed or piped in: one line without echo on a terminal, else
/// all of stdin with one trailing newline dropped.
fn read_stdin_value() -> Result<String> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			bevybail!("pass the value with `--value`: this host has no stdin to read it from")
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
