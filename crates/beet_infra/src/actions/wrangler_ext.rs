//! The `wrangler` cli, which is how everything Cloudflare-hosted is deployed.
//!
//! Cloudflare's own tool rather than terraform: a Worker's script, its bindings
//! and its custom domains are one upload, and the certificate for a custom
//! domain is issued by that upload. Splitting them would mean terraform owning
//! a record whose certificate wrangler owns, which is the one arrangement
//! guaranteed to fight itself.
use beet_core::prelude::*;

/// The Workers runtime compatibility date every project here pins. A date
/// rather than "latest": a Worker's behaviour is fixed by the date it declares,
/// so an unpinned one changes underneath a deploy nobody ran.
pub const COMPATIBILITY_DATE: &str = "2025-06-01";

/// The build directory for a Cloudflare project (`target/<name>-cf/`), created
/// if it is not there.
pub fn project_dir(name: &str) -> Result<AbsPathBuf> {
	let dir = AbsPathBuf::new_workspace_rel(".")?
		.join("target")
		.join(format!("{name}-cf"));
	fs_ext::create_dir_all(&dir)?;
	Ok(dir)
}

/// `wrangler deploy` from a project directory. When `secrets_file` is set, its
/// keys are uploaded as real Worker secrets *with* this version
/// (`--secrets-file`), which is the only way a deploy publishes secrets: a
/// `.dev.vars` file is a local-development input and never leaves the machine.
pub async fn deploy(
	project_dir: &AbsPathBuf,
	secrets_file: Option<&str>,
) -> Result {
	info!("wrangler deploy ({})", project_dir.display());
	let mut args = vec!["deploy".to_string()];
	if let Some(secrets_file) = secrets_file {
		args.push("--secrets-file".to_string());
		args.push(secrets_file.to_string());
	}
	ChildProcess::new("wrangler")
		.with_args(args)
		.with_cwd(project_dir.clone())
		.run_async()
		.await?;
	Ok(())
}
