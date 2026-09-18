//! `secrets/encrypt`: a plaintext file into an age file.

use super::VaultParams;
use super::write_recipients;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsEncrypt`], surfaced in `--help`.
#[derive(Reflect)]
struct EncryptParams {
	/// The plaintext file to encrypt; absent, stdin is read to its end.
	file: Option<String>,
	/// Who may read the file, comma separated `age1..,age1..`; absent, this
	/// identity file's own recipients.
	recipients: Option<String>,
}

/// Encrypt a plaintext file (or piped stdin) into the age file `--vault`
/// names, replacing what it held: a key, a certificate, any bytes. A secret
/// with a name and a role belongs in the secrets document instead. Editing
/// is `decrypt` to a file, edit, `encrypt` it back, delete the file.
///
/// ```sh
/// beet secrets/encrypt --vault=infra/cert.pem.age --file=cert.pem
/// cat id_ed25519 | beet secrets/encrypt --vault=~/keys/id_ed25519.age --recipients=age1..
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("encrypt"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, EncryptParams)>()
)]
pub async fn SecretsEncrypt(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<EncryptParams>()?;
	let plaintext = match &params.file {
		Some(file) => fs_ext::read(file)?,
		None => read_stdin()?,
	};
	let vault = VaultParams::resolve(&cx.input)?;
	let recipients = write_recipients(
		params.recipients.as_deref(),
		&AgeIdentityFile::require()?,
	)?;
	vault.write(&plaintext, &recipients).await?;
	Response::ok_text(format!(
		"encrypted {} bytes into {} ({} recipients)\n",
		plaintext.len(),
		vault.describe(),
		recipients.len()
	))
	.xok()
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
					`cat cert.pem | beet secrets/encrypt --vault=cert.pem.age`"
				);
			}
			let mut plaintext = Vec::new();
			std::io::stdin().read_to_end(&mut plaintext)?;
			plaintext.xok()
		}
	}
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Native only: the plaintext is read off the filesystem.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn encrypts_a_file_replacing_the_vault() {
		let dir = std::env::temp_dir()
			.join(format!("beet-encrypt-{}", Timestamp::now().millis()));
		let file = dir.join("cert.pem");
		fs_ext::write(&file, "A=file\n").unwrap();
		let mut world = VerbWorld::new();
		world.write("cert.pem.age", "A=old\nB=2\n").await;
		let uri = world.uri("cert.pem.age");
		world
			.call_str(
				SecretsEncrypt,
				Request::from_cli_str(&format!(
					"--vault={uri} --file={}",
					file.to_string_lossy()
				)),
			)
			.await
			.unwrap()
			.xpect_contains("encrypted 7 bytes into `cert.pem.age`");
		world
			.call_str(
				SecretsDecrypt,
				Request::from_cli_str(&format!("--vault={uri}")),
			)
			.await
			.unwrap()
			.xpect_eq("A=file\n");
		fs_ext::remove(&dir).unwrap();
	}
}
