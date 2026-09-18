//! `vault/encrypt`: a plaintext file into an age file.

use super::VaultParams;
use super::write_recipients;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`VaultEncrypt`], surfaced in `--help`.
#[derive(Reflect)]
struct EncryptParams {
	/// The plaintext file to encrypt; absent, stdin is read to its end.
	file: Option<String>,
	/// Encrypt to a group's recipients as a secrets document lists them:
	/// the declared document's, or `--document`'s.
	group: Option<String>,
	/// The document `--group` reads, a declared label or a path; absent,
	/// the declared document.
	document: Option<String>,
	/// Who may read the file, comma separated `age1..,age1..`; absent (and
	/// no `--group`), this identity file's own recipients.
	recipients: Option<String>,
}

/// Encrypt a plaintext file (or piped stdin) into the age file `--vault`
/// names, replacing what it held: a key, a certificate, any bytes. A secret
/// with a name and a role belongs in the secrets document instead. The
/// recipients are `--group`'s as a secrets document lists them (the one
/// reviewed recipient list in a repo, borrowed rather than retyped), else
/// `--recipients`, else this identity file's own. Editing is `decrypt` to a
/// file, edit, `encrypt` it back, delete the file.
///
/// ```sh
/// beet vault/encrypt --vault=infra/cert.pem.age --file=cert.pem --group=default
/// cat id_ed25519 | beet vault/encrypt --vault=~/keys/id_ed25519.age --recipients=age1..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("encrypt"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, EncryptParams)>()
)]
pub async fn VaultEncrypt(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<EncryptParams>()?;
	let plaintext = match &params.file {
		Some(file) => fs_ext::read(file)?,
		None => read_stdin()?,
	};
	let vault = VaultParams::resolve(&cx.input)?;
	let recipients = match &params.group {
		Some(group) => {
			if params.recipients.is_some() {
				bevybail!("pass `--group` or `--recipients`, not both");
			}
			group_recipients(&cx.caller, params.document.as_deref(), group)
				.await?
		}
		None => write_recipients(
			params.recipients.as_deref(),
			&AgeIdentityFile::require()?,
		)?,
	};
	vault.write(&plaintext, &recipients).await?;
	Response::ok_text(format!(
		"encrypted {} bytes into {} ({} recipients)\n",
		plaintext.len(),
		vault.describe(),
		recipients.len()
	))
	.xok()
}

/// The recipients of `group` in the document `selector` names.
async fn group_recipients(
	caller: &AsyncEntity,
	selector: Option<&str>,
	group: &str,
) -> Result<Vec<AgeRecipient>> {
	let handle = SecretsHandle::resolve(caller, selector).await?;
	let document = handle.read().await?;
	document
		.groups
		.get(group)
		.map(|group| group.recipients.clone())
		.ok_or_else(|| {
			bevyhow!(
				"no group `{group}` in {} (groups: {})",
				handle.describe(),
				document
					.group_names()
					.map(|name| format!("`{name}`"))
					.collect::<Vec<_>>()
					.join(", ")
			)
		})
}

/// All of stdin, which must be a pipe: a vault is a file, not a line typed
/// by hand.
fn read_stdin() -> Result<Vec<u8>> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			bevybail!("pass the plaintext with `--file`: this host has no stdin to read it from")
		} else {
			use std::io::IsTerminal;
			use std::io::Read;
			if std::io::stdin().is_terminal() {
				bevybail!(
					"pass the plaintext with `--file=<path>` or pipe it in, ie \
					`cat cert.pem | beet vault/encrypt --vault=cert.pem.age`"
				);
			}
			let mut plaintext = Vec::new();
			std::io::stdin().read_to_end(&mut plaintext)?;
			plaintext.xok()
		}
	}
}

// native only: the plaintext is read off the filesystem
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
	use super::super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn encrypts_a_file_replacing_the_vault() {
		let dir = std::env::temp_dir()
			.join(format!("beet-encrypt-{}", Timestamp::now().millis()));
		let file = dir.join("cert.pem");
		fs_ext::write(&file, "A=file\n").unwrap();
		let mut fixture = VerbWorld::new();
		fixture.write("cert.pem.age", "A=old\nB=2\n").await;
		let uri = fixture.uri("cert.pem.age");
		fixture
			.call_str(
				VaultEncrypt,
				Request::from_cli_str(&format!(
					"--vault={uri} --file={}",
					file.to_string_lossy()
				)),
			)
			.await
			.unwrap()
			.xpect_contains("encrypted 7 bytes into `cert.pem.age`");
		fixture
			.call_str(
				VaultDecrypt,
				Request::from_cli_str(&format!("--vault={uri}")),
			)
			.await
			.unwrap()
			.xpect_eq("A=file\n");

		// `--group`: a document's list, here a stranger's, locks this
		// identity out of its own file
		let stranger = AgeIdentity::generate();
		let mut strangers = AgeIdentityFile::default();
		strangers.push(stranger.clone());
		let mut document = fixture.document().await;
		document.groups.insert(
			"theirs".into(),
			SecretsGroup::new(vec![stranger.to_recipient()]),
		);
		fixture
			.secrets("secrets.toml")
			.write(&document)
			.await
			.unwrap();
		fixture
			.call_str(
				VaultEncrypt,
				Request::from_cli_str(&format!(
					"--vault={uri} --file={} --group=theirs",
					file.to_string_lossy()
				)),
			)
			.await
			.unwrap()
			.xpect_contains("(1 recipients)");
		fixture
			.vault("cert.pem.age")
			.read(&strangers)
			.await
			.unwrap()
			.xpect_eq(b"A=file\n".to_vec());
		fixture
			.call(
				VaultEncrypt,
				Request::from_cli_str(&format!(
					"--vault={uri} --file={} --group=nope",
					file.to_string_lossy()
				)),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`theirs`");
		fs_ext::remove(&dir).unwrap();
	}
}
