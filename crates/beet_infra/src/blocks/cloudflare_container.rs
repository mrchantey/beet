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
#[derive(Debug, Clone, Get, SetWith, Component)]
pub struct CloudflareContainerBlock {
	/// Worker + container name; the deployed host is
	/// `<name>.<account-subdomain>.workers.dev`.
	name: SmolStr,
	/// R2 bucket the container reads the site from (created on deploy).
	bucket: SmolStr,
	/// Port the container exposes + the fronting Worker proxies to. Must match the
	/// port the served site listens on (its markup `HttpServer{port}`), so it
	/// defaults to `beet_net`'s `DEFAULT_SERVER_PORT` (8337), the same default
	/// `bsx_site` declares and `FargateBlock` uses.
	port: u16,
	/// Scale-to-zero idle timeout (eg `5m`): the container sleeps after this.
	sleep_after: SmolStr,
	/// Maximum concurrent container instances.
	max_instances: u32,
	/// Extra literal env injected into the container, eg the remote-store config
	/// from `remote_env(..)`. `BEET_S3_ENDPOINT` + the R2 credentials are added by
	/// the deploy action from the process environment.
	env_vars: Vec<Variable>,
}

impl Default for CloudflareContainerBlock {
	fn default() -> Self {
		Self {
			name: "beet-container".into(),
			bucket: "beet-site".into(),
			// = beet_net DEFAULT_SERVER_PORT (8337); the served site's markup port.
			port: 8337,
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
}
