//! Cloudflare Workers deploy block.
//!
//! Like [`CloudflareContainerBlock`](crate::prelude::CloudflareContainerBlock)
//! this is wrangler-provisioned rather than terraform, but it deploys `beet-cli`
//! compiled to wasm as the Worker itself rather than a container. The Worker
//! reads the site from R2 through the native `worker::Bucket` binding (the only
//! wasm-compatible R2 access), so it needs no S3 credentials at runtime: the R2
//! bucket is bound to the Worker by name.
use crate::prelude::*;
use beet_core::prelude::*;

/// Configuration for deploying `beet-cli` (wasm) to a Cloudflare Worker: the
/// Worker, and the R2 bucket it serves the site from.
///
/// A [`StoreBlock`] whose root is `r2://<binding>`, the uri a Worker reads its
/// bucket through, and the repo store of its stack ([`RepoStoreBlock`]): the
/// Worker is nothing but a server of that bucket, so the deploy bakes
/// [`RepoStoreQuery::bootstrap`] into the Worker's `vars` and the Worker boots
/// from `BEET_REPO` exactly as a native binary boots from `--repo`.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(RepoStoreBlock)]
#[component(immutable, on_insert = ErasedStoreBlock::on_insert::<Self>,
	on_remove = ErasedStoreBlock::on_remove
)]
pub struct CloudflareWorkerBlock {
	/// Worker name; the deployed host is
	/// `<name>.<account-subdomain>.workers.dev`.
	name: SmolStr,
	/// R2 bucket bound to the Worker (created on deploy), read via the native
	/// `worker::Bucket` binding named [`binding`](Self::binding).
	bucket: SmolStr,
	/// The wrangler R2 binding name the Worker resolves its bucket from, ie the
	/// `SITE_BUCKET` in `env.SITE_BUCKET`. Written into the generated
	/// `wrangler.jsonc` and, as `r2://<binding>`, into the Worker's `BEET_REPO`
	/// var, so the two cannot drift.
	binding: SmolStr,
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
			binding: "SITE_BUCKET".into(),
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

impl Block for CloudflareWorkerBlock {
	/// The Worker name: wrangler names are the account's, composed by nothing.
	fn label(&self) -> &SmolStr { &self.name }
}

impl StoreBlock for CloudflareWorkerBlock {
	/// The bucket as the Worker reads it, through its binding rather than the
	/// S3-compatible api, so the uri names no account and no credential.
	fn store_uri(&self, _stack: &ResolvedStack) -> Result<StoreUri> {
		StoreUri::R2 {
			name: self.binding.clone(),
			path_prefix: None,
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The block declares the repo store: its erased half roots at the
	/// binding, and a consumer beside it bakes `BEET_REPO=r2://<binding>`.
	#[beet_core::test]
	fn declares_the_repo_store() {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let block = world
			.spawn(
				CloudflareWorkerBlock::new("hello")
					.with_bucket("hello-site")
					.with_binding("SITE"),
			)
			.id();
		world.flush();
		world
			.get::<ErasedStoreBlock>(block)
			.unwrap()
			.root()
			.to_string()
			.xpect_eq("r2://SITE");
		world
			.with_state::<RepoStoreQuery, _>(|repos| repos.bootstrap(block))
			.unwrap()
			.to_env()
			.xpect_eq(vec![(
				SmolStr::new("BEET_REPO"),
				SmolStr::new("r2://SITE"),
			)]);
	}
}
