//! The Cloudflare Worker entry: a wasm `#[event(fetch)]` that serves the no-code
//! BSX entry from an R2 bucket through the beet render router.
//!
//! The Worker boots exactly as the native binary does: its process config is
//! [`BootstrapConfig::get`], read off the environment, which is the Worker's
//! `vars` seeded into the agnostic env on each `fetch` ([`seed_env`]). The
//! deploy bakes `BEET_HOME=r2://<binding>` in there
//! (`HomeStoreQuery::bootstrap` on the `CloudflareWorkerBlock`), so the repo
//! store is a declaration on the deploy side and a uri on this side, with no
//! binding name in between. The per-isolate [`WorkerWorld`] is built (or
//! reused) through the shared core end to end: the same [`build_app`]
//! ([`BeetPlugins`] + [`WorkersPlugin`]), the same
//! [`entry_build::resolve_entry`] resolution and the same
//! [`entry_build::build_entry_owned`] build+settle every world-owning driver
//! uses. The glue that remains here is genuinely platform-specific: the env
//! seed, the worker request/response conversion, and the per-isolate world
//! cache with version invalidation.
//!
//! The entry builds into a root carrying the home store, disarmed via
//! `DisableCallOnReady` (so its declared servers stay down; the Worker itself
//! serves each request). The universal seam is the same
//! `entity.exchange(request) -> Response` the native servers use.

use crate::prelude::*;
use beet::prelude::*;
use worker::Context;
use worker::Env;
use worker::Request as WorkerRequest;
use worker::Response as WorkerResponse;
use worker::event;
use worker::js_sys;
use worker::wasm_bindgen::JsCast;

/// The Worker `fetch` handler: route an incoming request through the site world.
#[event(fetch)]
async fn fetch(
	req: WorkerRequest,
	env: Env,
	_ctx: Context,
) -> worker::Result<WorkerResponse> {
	console_error_panic_hook::set_once();

	// the Worker's config is its `vars`: seed them before the first
	// `BootstrapConfig::get()` memoizes, then stash the env so any binding (an
	// `R2WorkersStore`'s bucket, a `DurableObjectStore`'s namespace) resolves
	// for the duration of this invocation.
	seed_env(&env);
	WorkerEnv::set(env);

	// convert, route, convert back; map any beet error to a 500. `error!` reaches
	// `wrangler tail`: the site's `LogPlugin` installs a JS-console tracing
	// subscriber on wasm (see `PrettyTracing`), so the whole stack's diagnostics
	// surface, not just this entry.
	match handle(req).await {
		Ok(response) => Ok(response),
		Err(err) => {
			error!("worker fetch failed: {err}");
			WorkerResponse::error("Internal Server Error", 500)
		}
	}
}

/// Copy every string-valued var off the Worker's [`Env`] into the agnostic
/// environment, so `env_ext::var("BEET_HOME")` answers in a Worker exactly as
/// it does natively. Every var, not only `BEET_*`: the env is the process's,
/// and a site's own actions read theirs through the same surface. A binding
/// (a bucket, a namespace) is an object and is skipped.
fn seed_env(env: &Env) {
	let keys = js_sys::Object::keys(env.unchecked_ref::<js_sys::Object>());
	for key in (0..keys.length()).filter_map(|i| keys.get(i).as_string()) {
		if let Some(value) = js_sys::Reflect::get(env, &key.as_str().into())
			.ok()
			.and_then(|value| value.as_string())
		{
			// SAFETY: the Worker runtime is single-threaded, so nothing reads
			// the environment concurrently.
			unsafe { env_ext::set_var(&key, &value) }.ok();
		}
	}
}

/// Convert the request, route it through the (lazily built, version-checked)
/// entry world, and convert the response back.
async fn handle(req: WorkerRequest) -> Result<WorkerResponse> {
	let request = WorkerEnv::to_request(req).await?;
	let config = BootstrapConfig::get();

	// the store the deploy baked in, the one kind a Worker serves from: its
	// concrete provider heads the entry for the version check, its erased form
	// is what the entry resolves through.
	let home_uri = config.home.as_ref().ok_or_else(|| {
		bevyhow!(
			"no `BEET_HOME` var: the deploy bakes `r2://<binding>` into the \
			 Worker's vars"
		)
	})?;
	let BlobStoreProvider::R2(store) = BlobStoreProvider::from_uri(home_uri)?
	else {
		bevybail!(
			"`BEET_HOME={home_uri}` is not an `r2://<binding>` uri, the one \
			 store a Worker serves from"
		);
	};
	let entry_name = entry_build::self_rooted_entry_name(
		&BlobStore::new(store.clone()),
		config.entry.as_deref(),
	)
	.await?;

	// take the per-isolate world out so the exchange can borrow it mutably across
	// the await.
	let mut worker_world = WorkerWorld::take();

	// rebuild if absent or the bucket's entry version changed (a re-synced entry
	// reflects on the next request).
	let current_version = head_version(&store, &entry_name).await;
	let stale = worker_world
		.as_ref()
		.map(|loaded| loaded.version != current_version)
		.unwrap_or(true);
	if stale {
		worker_world = Some(
			build_worker_world(
				home_uri,
				config.store_fork.as_ref(),
				config.entry.as_deref(),
				current_version,
			)
			.await?,
		);
	}
	let mut worker_world = worker_world.expect("world built above");

	// route the request through the host entity's `Router` action; `exchange`
	// drives the app to completion (ticking the async executor) on the local thread.
	let response = worker_world
		.world
		.entity_mut(worker_world.host)
		.exchange(request)
		.await;
	let worker_response = WorkerEnv::to_worker_response(response).await;

	// put the world back for the next request.
	worker_world.put();
	worker_response
}

/// Build the per-isolate entry world from R2: take the native binary's
/// [`build_app`] ([`BeetPlugins`] + [`WorkersPlugin`]), resolve the entry
/// against the prescan set its plugins registered, run the shared
/// [`build_entry_owned`] build+settle, then resolve the host entity.
///
/// The build is disarmed (`DisableCallOnReady`): the Worker itself routes each
/// request through the host's `Router` action via `exchange`, so the servers the
/// entry declares (`HttpServer`, `TuiServer`, ...) must stay dormant. Armed, the
/// entry's `CallOnReady` verb would boot them on its `Ready`, and `HttpServer`'s
/// start would hit the (wasm-absent) backend and panic. Same disarmed build
/// `export-static`/`check` use.
async fn build_worker_world(
	home_uri: &StoreUri,
	store_fork: Option<&StoreUri>,
	entry: Option<&str>,
	version: Option<String>,
) -> Result<WorkerWorld> {
	// the same app the native binary builds, plus `WorkersPlugin`'s no-op runner
	// and per-isolate cell. `init` runs plugin `finish`/`cleanup` so deferred setup
	// lands before the build; the built world is then driven directly (the runner
	// never runs, since the Worker drives per-fetch).
	let mut app = build_app();
	app.init();
	let mut world = core::mem::take(app.world_mut());
	let prescans = world.get_resource_or_init::<PrescanRegistry>().clone();
	let resolved = entry_build::resolve_entry(
		&prescans,
		Some(home_uri),
		store_fork,
		entry,
	)
	.await?;
	let root = entry_build::build_entry_owned(&mut world, resolved).await?;

	// the host carries the `Router` action exchanges dispatch to: the url
	// space that serves pages, beneath any command routes the entry declares
	// (a `CliServer` root's `serve`), never merely the first router built.
	let host = world.run_system_cached_with::<_, Result<Entity>, _, _>(
		find_router,
		root,
	)??;

	WorkerWorld {
		world,
		host,
		version,
	}
	.xok()
}

/// The R2 object version of `path`, used as the rebuild marker. Returns `None`
/// when the head lookup fails or the object is absent, in which case every
/// request rebuilds (a safe, if slower, fallback).
async fn head_version(store: &R2WorkersStore, path: &str) -> Option<String> {
	store
		.head_version(&RelPath::from(path))
		.await
		.ok()
		.flatten()
}
