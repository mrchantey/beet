//! `secrets/restore-identity`: a backup's identities into the identity file.

use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// Request params for [`SecretsRestoreIdentity`], surfaced in `--help`.
#[derive(Reflect)]
struct RestoreIdentityParams {
	/// The backup `secrets/backup` wrote.
	file: String,
	/// The identity file to append to; defaults to the OS config,
	/// `~/.config/beet/age/keys.txt`.
	out: Option<String>,
	/// Refuse rather than prompt, for a runner with no terminal.
	headless: bool,
}

/// Restore a backup onto this machine: prompt for its passphrase, decrypt
/// it and append every identity it holds to the identity file, skipping
/// any already there. A new machine restores rather than generating a
/// second identity that could read nothing.
///
/// ```sh
/// beet secrets/restore-identity --file=/media/stick/beet-identity-2026-09-17.age
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("restore-identity"),
	ParamsPartial = ParamsPartial::new::<RestoreIdentityParams>()
)]
pub async fn SecretsRestoreIdentity(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let params = cx.input.parse_params::<RestoreIdentityParams>()?;
	let ciphertext = fs_ext::read(&params.file)?;
	if params.headless {
		bevybail!(
			"a backup passphrase is typed on a terminal: run this \
			interactively, without `--headless`"
		);
	}
	let passphrase = terminal_ext::read_secret_line("passphrase: ")?;
	let plaintext = AgePassphrase::new(passphrase).decrypt(&ciphertext)?;
	let restored = String::from_utf8(plaintext)
		.map_err(|_| bevyhow!("the backup is not an identity file"))?
		.xmap(|text| AgeIdentityFile::parse(&text))?;
	let path = match params.out {
		Some(out) => PathBuf::from(out),
		None => AgeIdentityFile::default_path()?,
	};
	let mut file = match fs_ext::exists(&path)? {
		true => AgeIdentityFile::read(&path)?,
		false => AgeIdentityFile::default(),
	};
	let present = file.recipients();
	let added = restored
		.identities()
		.filter(|identity| !present.contains(&identity.to_recipient()))
		.cloned()
		.collect::<Vec<_>>();
	for identity in &added {
		file.push(identity.clone());
	}
	file.write(&path)?;
	Response::ok_text(format!(
		"restored {} identities into `{}` ({} already there), recipients: {}\n",
		added.len(),
		path.display(),
		restored.len() - added.len(),
		restored
			.recipients()
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.join(", ")
	))
	.xok()
}
