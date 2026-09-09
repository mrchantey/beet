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

/// The Cloudflare v4 API base. `wrangler` uploads a Worker but has no verb for
/// removing one custom domain, so the teardown half of a Worker is REST.
const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// The account id + api token from the environment, the auth every workers call
/// needs.
fn account_env() -> Result<(SmolStr, SmolStr)> {
	let account = env_ext::var("CLOUDFLARE_ACCOUNT_ID")
		.map_err(|_| bevyhow!("CLOUDFLARE_ACCOUNT_ID is unset"))?;
	let token = env_ext::var("CLOUDFLARE_API_TOKEN")
		.map_err(|_| bevyhow!("CLOUDFLARE_API_TOKEN is unset"))?;
	Ok((account, token))
}

/// Delete the Worker custom domain serving `hostname`, if there is one.
///
/// `false` when there was none, which a teardown treats as success: it walks the
/// declaration rather than a ledger of what was created, so it routinely
/// addresses a resource that was never made.
///
/// Deleting the custom domain also removes the zone record and the certificate
/// wrangler provisioned with it, since the upload created all three together.
pub async fn delete_custom_domain(hostname: &str) -> Result<bool> {
	let (account, token) = account_env()?;
	let listed = send(
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
	send(
		beet_net::prelude::Request::delete(format!(
			"{API_BASE}/accounts/{account}/workers/domains/{id}"
		))
		.with_auth_bearer(&token),
		"deleting a worker custom domain",
	)
	.await
	.map(|found| found.is_some())
}

/// Delete the Worker script `name`, if there is one. `false` when there was
/// none, see [`delete_custom_domain`].
pub async fn delete_script(name: &str) -> Result<bool> {
	let (account, token) = account_env()?;
	send(
		beet_net::prelude::Request::delete(format!(
			"{API_BASE}/accounts/{account}/workers/scripts/{name}"
		))
		.with_auth_bearer(&token),
		"deleting a worker script",
	)
	.await
	.map(|found| found.is_some())
}

/// Send `request`, returning [`None`] when the thing it addressed is not there
/// and failing on any other non-2xx or `success: false` envelope.
async fn send(
	request: beet_net::prelude::Request,
	what: &str,
) -> Result<Option<serde_json::Value>> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await.unwrap_or_default();
	if status.as_u16() == 404 {
		return Ok(None);
	}
	// a `204` answers a delete with nothing at all, which is not an envelope
	if status.is_ok() && body.trim().is_empty() {
		return Ok(Some(serde_json::Value::Null));
	}
	let json: serde_json::Value =
		serde_json::from_str(&body).unwrap_or_default();
	if !status.is_ok() || json["success"] != true {
		bevybail!("{what} failed: {status} - {body}");
	}
	Ok(Some(json))
}
