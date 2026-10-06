//! The `wrangler` cli, which is how everything Cloudflare-hosted is deployed.
//!
//! Cloudflare's own tool rather than terraform: a Worker's script, its bindings
//! and its custom domains are one upload, and the certificate for a custom
//! domain is issued by that upload. Splitting them would mean terraform owning
//! a record whose certificate wrangler owns, which is the one arrangement
//! guaranteed to fight itself.
use crate::actions::cloudflare_api_ext;
use crate::actions::cloudflare_api_ext::API_BASE;
use crate::prelude::*;
use beet_core::prelude::*;

/// The Workers runtime compatibility date every project here pins. A date
/// rather than "latest": a Worker's behaviour is fixed by the date it declares,
/// so an unpinned one changes underneath a deploy nobody ran.
pub const COMPATIBILITY_DATE: &str = "2025-06-01";

/// The build directory for a Cloudflare project (`target/<name>-cf/`), created
/// if it is not there.
pub fn project_dir(name: &str) -> Result<AbsPath> {
	let dir = AbsPath::new_workspace_rel(".")?
		.join("target")
		.join(format!("{name}-cf"));
	fs_ext::create_dir_all(&dir)?;
	Ok(dir)
}

/// `wrangler deploy` from a project directory, under the deploy token the
/// calling action's `access` declares. When `secrets_file` is set, its keys are
/// uploaded as real Worker secrets *with* this version (`--secrets-file`),
/// which is the only way a deploy publishes secrets: a `.dev.vars` file is a
/// local-development input and never leaves the machine.
pub async fn deploy(
	access: &CloudflareAccess,
	project_dir: &AbsPath,
	secrets_file: Option<&str>,
) -> Result {
	info!("wrangler deploy ({})", project_dir);
	let mut args = vec!["deploy".to_string()];
	if let Some(secrets_file) = secrets_file {
		args.push("--secrets-file".to_string());
		args.push(secrets_file.to_string());
	}
	access
		.wrangler()
		.with_args(args)
		.with_cwd(project_dir.clone())
		.run_async()
		.await?;
	Ok(())
}

/// Delete the Worker custom domain serving `hostname` in `account`, if there
/// is one.
///
/// `false` when there was none, which a teardown treats as success: it walks the
/// declaration rather than a ledger of what was created, so it routinely
/// addresses a resource that was never made.
///
/// REST rather than wrangler, which has no verb for removing one custom domain.
///
/// Deleting the custom domain also removes the zone record and the certificate
/// wrangler provisioned with it, since the upload created all three together.
pub async fn delete_custom_domain(
	access: &CloudflareAccess,
	account: &CloudflareAccount,
	hostname: &str,
) -> Result<bool> {
	let (account, token) = (account.id(), access.token()?);
	let listed = cloudflare_api_ext::send_optional(
		beet_net::prelude::Request::get(format!(
			"{API_BASE}/accounts/{account}/workers/domains?hostname={hostname}"
		))
		.with_auth_bearer(&token),
		"listing worker custom domains",
	)
	.await?;
	let Some(id) = listed
		.and_then(|body| body["result"].as_array().cloned())
		.unwrap_or_default()
		.first()
		.and_then(|domain| domain["id"].as_str().map(String::from))
	else {
		return Ok(false);
	};
	cloudflare_api_ext::send_optional(
		beet_net::prelude::Request::delete(format!(
			"{API_BASE}/accounts/{account}/workers/domains/{id}"
		))
		.with_auth_bearer(&token),
		"deleting a worker custom domain",
	)
	.await
	.map(|found| found.is_some())
}

/// Delete the Worker script `name` in `account`, if there is one. `false`
/// when there was none, see [`delete_custom_domain`].
pub async fn delete_script(
	access: &CloudflareAccess,
	account: &CloudflareAccount,
	name: &str,
) -> Result<bool> {
	let (account, token) = (account.id(), access.token()?);
	cloudflare_api_ext::send_optional(
		beet_net::prelude::Request::delete(format!(
			"{API_BASE}/accounts/{account}/workers/scripts/{name}"
		))
		.with_auth_bearer(&token),
		"deleting a worker script",
	)
	.await
	.map(|found| found.is_some())
}
