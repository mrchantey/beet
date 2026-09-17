//! `secrets/import`: a plaintext file merged into a vault.

use super::VaultParams;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsImport`], surfaced in `--help`.
#[derive(Reflect)]
struct ImportParams {
	/// A plaintext file, its format by extension (`.env`, `.toml`, `.json`),
	/// every entry merged into the vault.
	file: String,
	/// Overwrite keys the vault already holds; by default an existing key is
	/// kept.
	replace: bool,
}

/// Merge a plaintext file into a vault: the `.env` migration
/// (`beet secrets/import --file=.env`, then delete `.env`), or any file in
/// one of the vault formats. An existing key is kept unless `--replace`.
///
/// ```sh
/// beet secrets/import --file=.env
/// beet secrets/import --file=export.toml --vault=mail-prod --replace
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("import"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, ImportParams)>()
)]
pub async fn SecretsImport(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<ImportParams>()?;
	let format = VaultFormat::from_plaintext_path(&params.file)?;
	let other =
		VaultDocument::parse(format, &fs_ext::read_to_string(&params.file)?)?;
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let identities = AgeIdentityFile::require()?;
	let mut doc = vault.read_or_empty(&identities).await?;
	let written = doc.merge(&other, params.replace)?;
	let recipients = vault.write_recipients(&identities)?;
	vault.write(&doc, &recipients).await?;
	Response::ok_text(format!(
		"imported {} of {} key(s) from `{}` into vault {} ({} recipients)\n",
		written.len(),
		other.keys().len(),
		params.file,
		vault.describe(),
		recipients.len()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Native only: the file is read off the filesystem.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn merges_without_clobbering_unless_replacing() {
		let dir = std::env::temp_dir()
			.join(format!("beet-import-{}", Timestamp::now().millis()));
		let file = dir.join(".env");
		fs_ext::write(&file, "A=file\nB=2\n").unwrap();
		let mut world = VerbWorld::new();
		world.set(".env", "A", "vault").await;
		let request = |extra: &str| {
			Request::from_cli_str(&format!(
				"--file={} {extra}",
				file.to_string_lossy()
			))
		};
		world
			.call_str(SecretsImport, request(""))
			.await
			.unwrap()
			.xpect_contains("imported 1 of 2");
		world
			.call_str(SecretsGet, Request::get("/").with_param("key", "A"))
			.await
			.unwrap()
			.xpect_eq("vault");
		world
			.call_str(SecretsImport, request("--replace"))
			.await
			.unwrap()
			.xpect_contains("imported 2 of 2");
		world
			.call_str(SecretsGet, Request::get("/").with_param("key", "A"))
			.await
			.unwrap()
			.xpect_eq("file");
		fs_ext::remove(&dir).unwrap();
	}
}
