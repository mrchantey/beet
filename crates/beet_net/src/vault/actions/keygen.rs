//! `vault/keygen`: a new identity into the identity file.

use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// Request params for [`VaultKeygen`], surfaced in `--help`.
#[derive(Reflect)]
struct KeygenParams {
	/// The identity file to append to; defaults to the OS config,
	/// `~/.config/beet/age/keys.txt`.
	out: Option<String>,
}

/// Make a new age identity and append it to the identity file, created
/// owner-only when missing, printing its recipient and the next steps. Never
/// overwrites: a second call appends a second identity. The identity itself
/// is never printed; `vault/backup` is how it leaves the machine.
///
/// ```sh
/// beet vault/keygen
/// beet vault/keygen --out=/tmp/keys.txt
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("keygen"),
	ParamsPartial = ParamsPartial::new::<KeygenParams>()
)]
pub async fn VaultKeygen(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<KeygenParams>()?;
	let path = match params.out {
		Some(out) => PathBuf::from(out),
		None => AgeIdentityFile::default_path()?,
	};
	let mut file = match fs_ext::exists(&path)? {
		true => AgeIdentityFile::read(&path)?,
		false => AgeIdentityFile::default(),
	};
	let identity = AgeIdentity::generate();
	let recipient = identity.to_recipient();
	file.push(identity);
	file.write(&path)?;
	info!(
		"appended an identity to {} ({} there now)",
		path.display(),
		file.len()
	);
	Response::ok_text(format!(
		"recipient: {recipient}\n\
		identity file: {} ({} identities)\n\
		\n\
		Next:\n\
		1. back it up: `beet vault/backup --qr` onto a stick, and remember \
		the passphrase\n\
		2. add the recipient wherever it should read: a secrets document's \
		groups, or `--recipients` on `vault/encrypt` and `vault/rekey`\n\
		3. `beet secrets/check`\n",
		path.display(),
		file.len()
	))
	.xok()
}

// native only: the file lands in the system temp dir
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
	use super::super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Two calls append two identities, neither printed.
	#[beet_core::test]
	async fn appends_and_never_overwrites() {
		let dir = std::env::temp_dir()
			.join(format!("beet-keygen-{}", Timestamp::now().millis()));
		let path = dir.join("keys.txt");
		let mut fixture = VerbWorld::new();
		for count in 1..=2 {
			let text = fixture
				.call_str(
					VaultKeygen,
					Request::from_cli_str(&format!(
						"--out={}",
						path.to_string_lossy()
					)),
				)
				.await
				.unwrap();
			text.as_str()
				.xpect_contains("recipient: age1")
				.xpect_contains(format!("({count} identities)"))
				.xnot()
				.xpect_contains(AgeIdentity::PREFIX);
			AgeIdentityFile::read(&path).unwrap().len().xpect_eq(count);
		}
		fs_ext::remove(&dir).unwrap();
	}
}
