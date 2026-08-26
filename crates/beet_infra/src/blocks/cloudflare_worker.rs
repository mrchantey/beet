//! Cloudflare Workers deploy block.
//!
//! Like [`CloudflareContainerBlock`](crate::prelude::CloudflareContainerBlock)
//! this is a plain config component (wrangler-provisioned, not terraform), but it
//! deploys `beet-cli` compiled to wasm as the Worker itself rather than a
//! container. The Worker reads the site from R2 through the native
//! `worker::Bucket` binding (the only wasm-compatible R2 access), so it needs no
//! S3 credentials at runtime: the R2 bucket is bound to the Worker by name.
use crate::prelude::*;
use beet_core::prelude::*;

/// The wrangler R2 binding name the wasm Worker resolves its `Bucket` from, kept
/// in sync with `beet-cli`'s `worker_entry` (`SITE_BUCKET_BINDING`). The
/// `#[event(fetch)]` entry reads `env.SITE_BUCKET` to get the live bucket handle.
pub(crate) const WORKER_R2_BINDING: &str = "SITE_BUCKET";

/// Configuration for deploying `beet-cli` (wasm) to a Cloudflare Worker.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CloudflareWorkerBlock {
	/// Worker name; the deployed host is
	/// `<name>.<account-subdomain>.workers.dev`.
	name: SmolStr,
	/// R2 bucket bound to the Worker (created on deploy), read via the native
	/// `worker::Bucket` binding named [`WORKER_R2_BINDING`].
	bucket: SmolStr,
	/// Plain `vars` injected into the Worker (non-secret), eg a deploy id.
	env_vars: Vec<Variable>,
	/// Hostnames this Worker answers on, each deployed as a wrangler *custom
	/// domain*: wrangler creates the zone record and the edge certificate for
	/// it, so the name is served over https with nothing else to declare. Empty
	/// leaves the Worker reachable only at its `workers.dev` host.
	///
	/// The record is therefore wrangler's, not terraform's, which is the one
	/// place in this stack where dns is not owned by the block that needs it.
	/// A zone audit has to allow for them.
	#[set_with(skip)]
	routes: Vec<SmolStr>,
}

impl Default for CloudflareWorkerBlock {
	fn default() -> Self {
		Self {
			name: "beet-worker".into(),
			bucket: "beet-site".into(),
			env_vars: Vec::new(),
			routes: Vec::new(),
		}
	}
}

impl CloudflareWorkerBlock {
	/// Create a block for the given worker name.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			..default()
		}
	}

	/// Serve this Worker at `host` as a custom domain, ie
	/// `mta-sts.stalwart.beetmash.com`. See [`routes`](Self::routes).
	pub fn with_route(mut self, host: impl Into<SmolStr>) -> Self {
		self.routes.push(host.into());
		self
	}
}
