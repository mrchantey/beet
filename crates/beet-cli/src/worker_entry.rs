//! The Cloudflare Worker entry: a wasm `#[event(fetch)]` that serves the no-code
//! BSX site from an R2 bucket through the beet render router.
//!
//! On each `fetch` the request's [`worker::Env`] is stashed so an
//! [`R2WorkersStore`] can resolve its live bucket binding, then the per-isolate
//! [`World`] is built (or reused) and the request is routed through it. Building
//! mirrors the native `load_entry` path, but every store read is awaited rather
//! than blocked on (the Worker runtime is single-threaded): templates register
//! via [`read_site_templates`]/[`register_site_templates`], the entry builds into a
//! root carrying the site store plus [`DisableBootOnLoad`] (so its declared servers
//! stay dormant; the Worker
//! itself serves each request), and the `<RoutesDir/>` discovery observer scans the
//! store as an async task, which the build settles via
//! [`AsyncRunner::settle_async_tasks`] before serving. The universal seam is the
//! same `entity.exchange(request) -> Response` the native servers use.

use beet::prelude::*;
use std::cell::RefCell;
use worker::Context;
use worker::Env;
use worker::Request as WorkerRequest;
use worker::Response as WorkerResponse;
use worker::event;

/// The R2 binding name the site bucket is bound to in `wrangler.toml`.
const SITE_BUCKET_BINDING: &str = "SITE_BUCKET";
/// The entry document at the bucket root.
const ENTRY_NAME: &str = "main.bsx";

thread_local! {
	/// The per-isolate built site [`App`], reused across requests. Taken out for
	/// the duration of an exchange (so the exchange can borrow its world mutably
	/// across an await) and put back after, keyed alongside the loaded site version
	/// so a re-synced bucket rebuilds on the next request.
	static SITE: RefCell<Option<LoadedSite>> = const { RefCell::new(None) };
}

/// A built site app plus the R2 version of the entry document it was built from,
/// so a changed bucket triggers a rebuild on the next request.
struct LoadedSite {
	app: App,
	/// The host entity carrying the `Router` action exchanges dispatch to.
	host: Entity,
	/// The R2 object version of `main.bsx` at build time, or `None` if the head
	/// check was unavailable (then every request rebuilds).
	version: Option<String>,
}

/// The Worker `fetch` handler: route an incoming request through the site world.
#[event(fetch)]
async fn fetch(
	req: WorkerRequest,
	env: Env,
	_ctx: Context,
) -> worker::Result<WorkerResponse> {
	console_error_panic_hook::set_once();

	// stash the env so any `R2WorkersStore` resolves its live bucket binding for
	// the duration of this invocation.
	let store = R2WorkersStore::new(SITE_BUCKET_BINDING);
	set_worker_env(env);

	// convert, route, convert back; map any beet error to a 500. `error!` reaches
	// `wrangler tail`: the site's `LogPlugin` installs a JS-console tracing
	// subscriber on wasm (see `PrettyTracing`), so the whole stack's diagnostics
	// surface, not just this entry.
	match handle(req, store).await {
		Ok(response) => Ok(response),
		Err(err) => {
			error!("worker fetch failed: {err}");
			WorkerResponse::error("Internal Server Error", 500)
		}
	}
}

/// Convert the request, route it through the (lazily built, version-checked)
/// site world, and convert the response back.
async fn handle(
	req: WorkerRequest,
	store: R2WorkersStore,
) -> Result<WorkerResponse> {
	let request = worker_to_request(req).await?;

	// take the world out so the exchange can borrow it mutably across the await.
	let mut site = SITE.with(|slot| slot.borrow_mut().take());

	// rebuild if absent or the bucket's entry version changed (a re-synced site
	// reflects on the next request).
	let current_version = head_version(&store, ENTRY_NAME).await;
	let stale = site
		.as_ref()
		.map(|loaded| loaded.version != current_version)
		.unwrap_or(true);
	if stale {
		site = Some(build_site(store, current_version).await?);
	}
	let mut site = site.expect("site built above");

	// route the request through the host entity's `Router` action; `exchange`
	// drives the app to completion (ticking the async executor) on the local thread.
	let response = site
		.app
		.world_mut()
		.entity_mut(site.host)
		.exchange(request)
		.await;
	let worker_response = response_to_worker(response).await;

	// put the world back for the next request.
	SITE.with(|slot| *slot.borrow_mut() = Some(site));
	worker_response
}

/// Build the site world from R2: add the serve plugins, register templates, build
/// the entry through the [`TemplateLoader`], spawn the `<RoutesDir/>` routes, and
/// resolve the host entity. Mirrors the native `load_entry` path, fully async.
async fn build_site(
	store: R2WorkersStore,
	version: Option<String>,
) -> Result<LoadedSite> {
	let mut app = App::new();
	// the same trusted defaults the native binary uses; on wasm `BeetPlugins`
	// resolves to the headless runner + the render router stack.
	app.add_plugins(BeetPlugins);
	// run plugin `finish`/`cleanup` so deferred plugin setup lands before the build.
	app.finish();
	app.cleanup();
	let world = app.world_mut();

	// the site store the R2 bucket backs; the entry, `templates/`, `<RoutesDir/>`
	// and `<Template src>` all resolve through it (composed on the root below).
	let store = BlobStore::new(store);
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	// read the `templates/` and entry document through the store (awaited, never
	// blocked), then build the entry into a root carrying the site store plus
	// `DisableBootOnLoad`: the Worker itself routes each request through the host's
	// `Router` action via `exchange`, so the servers the site's `main.bsx` declares
	// (`HttpServer`, `TuiServer`, ...) must stay dormant. Without `DisableBootOnLoad`
	// the entry's `BootOnLoad` verb boots them on `LoadTemplate`, and `HttpServer`'s
	// start hits the (wasm-absent) backend and panics. Same suppression
	// `export-static`/`check` use.
	//
	// the build's `Insert, RoutesDir` observer queues the route discovery (a store
	// scan) as an async task, settled below before the host is served.
	let sources = read_site_sources(&store, formats, ENTRY_NAME).await?;
	build_site_root(world, store, sources, DisableBootOnLoad)?;
	// settle the async runtime so the discovered routes (and any other boot-time
	// async) land before the host is queried and served.
	AsyncRunner::settle_async_tasks(world).await;

	// the host carries the `Router` action exchanges dispatch to.
	let host = world
		.query_filtered::<Entity, With<Router>>()
		.iter(world)
		.next()
		.ok_or_else(|| bevyhow!("no `Router` host found in loaded site"))?;

	LoadedSite { app, host, version }.xok()
}

/// The R2 object version of `path`, used as the rebuild marker. Returns `None`
/// when the head lookup fails or the object is absent, in which case every
/// request rebuilds (a safe, if slower, fallback).
async fn head_version(store: &R2WorkersStore, path: &str) -> Option<String> {
	store
		.head_version(&SmolPath::from(path))
		.await
		.ok()
		.flatten()
}

/// Convert a [`worker::Request`] into a beet [`Request`]: method, full URL,
/// headers, and the body bytes.
async fn worker_to_request(mut req: WorkerRequest) -> Result<Request> {
	let url = req.url()?;
	// `worker::Method` displays as its uppercase name, which `HttpMethod` parses.
	let method = req.method().to_string().parse::<HttpMethod>()?;
	// read the body bytes up front (the Worker request is consumed once).
	let body = req.bytes().await.unwrap_or_default();

	let mut parts = RequestParts::new(method, Url::parse(url.as_str()));
	for (key, value) in req.headers() {
		parts.headers.set_raw(key, value);
	}
	let body = match body.is_empty() {
		true => Body::default(),
		false => Body::Bytes(body.into()),
	};
	Request::from_parts(parts, body).xok()
}

/// Convert a beet [`Response`] into a [`worker::Response`]: collect the body
/// bytes, then carry the status and headers across.
async fn response_to_worker(response: Response) -> Result<WorkerResponse> {
	let (parts, body) = response.into_parts();
	let bytes = body.into_bytes().await?;
	let mut worker_response = WorkerResponse::from_bytes(bytes.to_vec())?
		.with_status(parts.status().as_u16());
	let headers = worker_response.headers_mut();
	for (key, values) in parts.headers.iter_all() {
		for value in values {
			headers.append(key, value)?;
		}
	}
	worker_response.xok()
}
