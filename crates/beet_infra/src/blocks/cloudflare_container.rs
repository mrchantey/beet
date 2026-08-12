//! Cloudflare Containers deploy block.
//!
//! Unlike the AWS blocks, this is NOT a terraform [`Block`](crate::prelude::Block):
//! Cloudflare is provisioned with the `wrangler` CLI (first-class
//! `r2 bucket`/`deploy`/`delete`), not OpenTofu, so the block is a plain config
//! component the Cloudflare deploy actions read (mirroring how
//! [`BuildDockerImageAction`](crate::prelude::BuildDockerImageAction) reads its
//! sibling [`BuildDockerImage`](crate::prelude::BuildDockerImage)).
//!
//! The surface mirrors [`FargateBlock`](crate::prelude::FargateBlock) (a name,
//! a port, instance bounds, and an `env_vars` list) so an example toggles between
//! Fargate and Cloudflare Containers by swapping the block + the deploy action.
//! It deploys the same native `beet` binary, run in a container that reads the
//! site from R2 at request time via [`S3Store::r2`](crate::prelude::S3Store).
use crate::prelude::*;
use beet_core::prelude::*;

/// Configuration for deploying the native `beet` binary to Cloudflare Containers.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CloudflareContainerBlock {
	/// Worker + container name; the deployed host is
	/// `<name>.<account-subdomain>.workers.dev`.
	name: SmolStr,
	/// R2 bucket the container reads the site from (created on deploy).
	bucket: SmolStr,
	/// Explicit port the container exposes and the fronting Worker proxies to. When
	/// `None`, resolved from `--port` / `BEET_HTTP_PORT` or
	/// [`DEFAULT_HTTP_PORT`](beet_net::prelude::DEFAULT_HTTP_PORT) (8337) via
	/// [`port`](Self::port). Must match the served site's markup `HttpServer{port}`
	/// (the same default `bsx_site` declares and `FargateBlock` uses).
	#[get(skip)]
	#[set_with(unwrap_option)]
	app_port: Option<u16>,
	/// Scale-to-zero idle timeout (eg `5m`): the container sleeps after this.
	sleep_after: SmolStr,
	/// Maximum concurrent container instances.
	max_instances: u32,
	/// Extra literal env injected into the container. The entry-store selection is
	/// *not* env: the deploy action bakes `--store=s3://<bucket>?endpoint=..` into
	/// the image `CMD`; only the R2 credentials (SDK convention) ride env.
	env_vars: Vec<Variable>,
}

impl Default for CloudflareContainerBlock {
	fn default() -> Self {
		Self {
			name: "beet-container".into(),
			bucket: "beet-site".into(),
			app_port: None,
			sleep_after: "5m".into(),
			max_instances: 3,
			env_vars: Vec::new(),
		}
	}
}

impl CloudflareContainerBlock {
	/// Create a block for the given worker/container name.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			..default()
		}
	}

	/// The resolved port the container exposes and the fronting Worker proxies to:
	/// the explicit [`app_port`](Self::with_app_port) if set, else `--port` /
	/// `BEET_HTTP_PORT`, else
	/// [`DEFAULT_HTTP_PORT`](beet_net::prelude::DEFAULT_HTTP_PORT) (8337).
	pub fn port(&self) -> u16 {
		beet_net::prelude::resolve_server_port(self.app_port)
	}

	/// The deployed binary's argv config, baked into the image `CMD`: the R2
	/// store uri (bucket + account `endpoint`, both known only at deploy time)
	/// constrained to the http transport.
	pub fn cmd_bootstrap(&self, endpoint: &str) -> Result<BootstrapConfig> {
		BootstrapConfig {
			store: Some(StoreUri::parse(&format!(
				"s3://{}?endpoint={endpoint}",
				self.bucket
			))?),
			server: Some(ServerFilter::new("http")),
			..default()
		}
		.xok()
	}

	/// The deployed binary's env config, rendered into the fronting Worker's
	/// `envVars`. `0.0.0.0` binds all interfaces: the Worker proxies to the
	/// container's own IP, so a localhost bind would be unreachable.
	pub fn runtime_bootstrap(&self) -> BootstrapConfig {
		BootstrapConfig {
			host: Some(core::net::Ipv4Addr::UNSPECIFIED.into()),
			..default()
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The image `CMD` is a real JSON array carrying only the argv channel: the
	/// R2 store uri and the transport selection, with no `serve` positional and no
	/// `--path=/` workaround (an empty boot path already opens `/`).
	#[beet_core::test]
	fn renders_cmd_json() {
		CloudflareContainerBlock::new("beet-hello")
			.cmd_bootstrap("https://acc.r2.cloudflarestorage.com")
			.unwrap()
			.to_cmd_json("/app")
			.unwrap()
			.xpect_eq(
				r#"["/app", "--store=s3://beet-site?endpoint=https://acc.r2.cloudflarestorage.com", "--server=http"]"#,
			);
	}

	/// The Worker's `envVars` carry the bind host under the name the runtime
	/// parses, and never a secret (those read from `this.env`).
	#[beet_core::test]
	fn renders_worker_env() {
		CloudflareContainerBlock::default()
			.runtime_bootstrap()
			.to_env()
			.xpect_eq(vec![(SmolStr::from("BEET_HOST"), SmolStr::from("0.0.0.0"))]);
	}
}
