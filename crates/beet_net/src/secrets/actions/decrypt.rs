//! `secrets/decrypt`: an age file's plaintext, printed.

use super::VaultParams;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsDecrypt`], surfaced in `--help`.
#[derive(Reflect)]
struct DecryptParams {
	/// Write the plaintext to this file, owner-only, instead of printing it:
	/// the way out for a file that is not text.
	out: Option<String>,
}

/// Print an age file's plaintext. This is the deliberate print: the
/// plaintext is the response, so `beet secrets/decrypt --vault=x.age > x`
/// lands it wherever a shell sends stdout, and `--out` writes it to an
/// owner-only file instead. Mind what is recording your session.
///
/// ```sh
/// beet secrets/decrypt --vault=infra/cert.pem.age
/// beet secrets/decrypt --vault=~/keys/id_ed25519.age --out=/tmp/id_ed25519
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("decrypt"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, DecryptParams)>()
)]
pub async fn SecretsDecrypt(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<DecryptParams>()?;
	let vault = VaultParams::resolve(&cx.input)?;
	let plaintext = vault.read(&AgeIdentityFile::require()?).await?;
	match params.out {
		Some(out) => {
			fs_ext::write_private(&out, &plaintext)?;
			Response::ok_text(format!(
				"decrypted {} into `{out}` ({} bytes)\n",
				vault.describe(),
				plaintext.len()
			))
		}
		None => String::from_utf8(plaintext)
			.map_err(|_| {
				bevyhow!(
					"{} is not text: `--out=<file>` writes its bytes",
					vault.describe()
				)
			})?
			.xmap(Response::ok_text),
	}
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn prints_text_and_refuses_bytes() {
		let mut world = VerbWorld::new();
		world.write("a.txt.age", "TOKEN=x\n").await;
		world
			.call_str(
				SecretsDecrypt,
				Request::from_cli_str(&format!(
					"--vault={}",
					world.uri("a.txt.age")
				)),
			)
			.await
			.unwrap()
			.xpect_eq("TOKEN=x\n");
		world
			.vault("b.age")
			.write(&[0xff, 0xfe], &[world.identity.to_recipient()])
			.await
			.unwrap();
		world
			.call(
				SecretsDecrypt,
				Request::from_cli_str(&format!(
					"--vault={}",
					world.uri("b.age")
				)),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("--out");
	}

	/// Native only: `--out` writes the filesystem.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn writes_out_owner_only() {
		let dir = std::env::temp_dir()
			.join(format!("beet-decrypt-{}", Timestamp::now().millis()));
		let out = dir.join("mail.toml");
		let mut world = VerbWorld::new();
		world.write("mail.toml.age", "a = 1\n").await;
		world
			.call_str(
				SecretsDecrypt,
				Request::from_cli_str(&format!(
					"--vault={} --out={}",
					world.uri("mail.toml.age"),
					out.to_string_lossy()
				)),
			)
			.await
			.unwrap()
			.xpect_contains("6 bytes");
		fs_ext::read_to_string(&out).unwrap().xpect_eq("a = 1\n");
		fs_ext::remove(&dir).unwrap();
	}
}
