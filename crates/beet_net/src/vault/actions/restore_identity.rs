//! `vault/restore-identity`: a backup's identities into the identity file.

use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

/// Restore a backup onto this machine: prompt for its passphrase, decrypt
/// it and append every identity it holds to the identity file, skipping
/// any already there. A new machine restores rather than generating a
/// second identity that could read nothing.
///
/// ```sh
/// beet vault/restore-identity --file=/media/stick/beet-identity-2026-09-17.age
/// ```
#[action(route = "restore-identity")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn VaultRestoreIdentity(
	/// The backup `vault/backup` wrote.
	#[param]
	file: SmolPath,
	/// The identity file to append to; defaults to the OS config,
	/// `~/.config/beet/age/keys.txt`.
	#[param]
	out: Option<SmolPath>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let ciphertext = fs_ext::read(&file)?;
	let passphrase =
		OperatorPrompt::ask(VaultBackup::passphrase_ask("passphrase: "))
			.await?;
	let plaintext = AgePassphrase::new(passphrase).decrypt(&ciphertext)?;
	let restored = String::from_utf8(plaintext)
		.map_err(|_| bevyhow!("the backup is not an identity file"))?
		.xmap(|text| AgeIdentityFile::parse(&text))?;
	let path = match out {
		Some(out) => out.to_path_buf(),
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
