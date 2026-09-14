//! Serving files from a [`BlobStore`] as static-file routes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment name [`ServeBlobs`] captures the trailing file path into.
/// Private: callers give [`ServeBlobs`] a mount prefix, never this capture name.
pub(crate) const STORE_PATH_PARAM: &str = "store_path";

/// Mounts a static-file route under `prefix`, serving the captured path from the
/// nearest self-or-ancestor [`BlobStore`].
///
/// `prefix` is the mount path, and the *only* thing it scopes: the greedy capture
/// excludes it, so an unpaired `<ServeBlobs prefix="assets"/>` serves the store
/// *root* under `/assets`. Serving a subdir is composition: pair with a co-located
/// [`DirPath`] scoping the inherited store (which is exactly [`AssetsDir`]), or
/// co-locate a store of its own. The greedy trailing capture and the
/// request/response adapter are private details ServeBlobs inserts for itself.
///
/// The mount is also a store another process reads: `<ServeBlobs prefix="repo"/>`
/// publishes a site's repo store, and an `HttpStore` at `/repo` (a browser the
/// site's page boots, a terminal pulling the site) reads it key for key. Its
/// listing rides the same route under `json`: `GET /repo/docs?list` answers the
/// keys under `docs` as a JSON array, relative to it.
#[template]
pub fn ServeBlobs(
	/// The mount path the static files are served under, eg `assets`.
	#[prop]
	prefix: String,
	/// Mark served files cacheable with this browser TTL (eg `cache="1h"`); the
	/// edge TTL is the [`CacheHeaders`] default, refreshed early by a deploy
	/// purge. Unset serves with no cache header (a live dev mount).
	cache: Option<Duration>,
) -> impl Bundle {
	(
		PathPartial::new(format!("{prefix}/*{STORE_PATH_PARAM}?")),
		ServeBlobsHandler,
		route::exchange_overload::<RequestParts, Response, _, _>(),
		OnSpawn::new(move |entity: &mut EntityWorldMut| {
			if let Some(browser_max_age) = cache {
				entity.insert(CacheHeaders {
					browser_max_age,
					..CacheHeaders::assets()
				});
			}
		}),
	)
}

/// The static-file handler behind [`ServeBlobs`]: serves the greedy
/// [`STORE_PATH_PARAM`] capture from the nearest self-or-ancestor [`BlobStore`].
///
/// Resolves the store by composition rather than constructing one, so any adjacent
/// store (filesystem, S3, in-memory) backs the route. A co-located [`DirPath`] has
/// already scoped that store to its subdir, so this just reads the resolved store;
/// serving rules mirror a static host (see [`serve_blob`]).
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub(crate) async fn ServeBlobsHandler(
	cx: ActionContext<RequestParts>,
) -> Result<Response> {
	// the nearest self-or-ancestor store (a `DirPath` co-located on this route has
	// already scoped it to the served subdir).
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
			|entity, stores| stores.get(entity).cloned(),
		)
		.await??;
	let path = cx
		.input
		.get_params(STORE_PATH_PARAM)
		.map(|segments| RelPath::from_segments(segments))
		.unwrap_or_else(|| RelPath::from(cx.input.path()));
	// the listing endpoint an `HttpStore` reads the mount's keys through
	#[cfg(feature = "json")]
	if cx.input.has_param(HttpStore::LIST_PARAM) {
		return list_blobs(&store, &path).await;
	}
	serve_blob(&store, &path)
		.await
		.map_err(|err| unhydrated_hint(&cx.input, err))
}

/// The keys under `path` as a JSON array, relative to it: the mount's root
/// lists every key, a subdir the keys beneath it.
#[cfg(feature = "json")]
async fn list_blobs(store: &BlobStore, path: &RelPath) -> Result<Response> {
	match path.as_str().is_empty() {
		true => store.list().await?,
		false => store.with_subdir(path.clone()).list().await?,
	}
	.xmap(|mut keys| {
		keys.sort();
		keys
	})
	.xref()
	.xmap(Response::ok_json)
}

/// The request path segment whose misses earn the unhydrated-store hint.
const HINT_DIR: &str = "assets";

/// Appends a pull hint to a miss under a top-level `assets/` request path.
///
/// A store-agnostic nudge rather than repo-specific knowledge: `assets`
/// directories are commonly synced from a blob bucket instead of committed, so a
/// miss there is usually an unhydrated checkout rather than a wrong url. Any
/// other status or path passes through untouched.
fn unhydrated_hint(request: &RequestParts, err: BevyError) -> BevyError {
	let miss = err
		.downcast_ref::<HttpError>()
		.filter(|err| err.status_code == StatusCode::NOT_FOUND)
		.filter(|_| {
			request.path().first().map(SmolStr::as_str) == Some(HINT_DIR)
		})
		.cloned();
	let note = format!(
		"note: '{HINT_DIR}' directories are commonly synced from a blob store; this checkout may need a pull"
	);
	match miss {
		Some(miss) => HttpError::new(
			miss.status_code,
			[miss.message.as_str(), note.as_str()]
				.into_iter()
				.filter(|part| !part.is_empty())
				.collect::<Vec<_>>()
				.join("\n"),
		)
		.into(),
		None => err,
	}
}

/// Mounts a named directory as static-file routes: serves the files under `src`
/// (resolved against the nearest ancestor [`BlobStore`]) at the url `prefix`.
///
/// Pairs [`ServeBlobs`] (the mount path) with a [`DirPath`] (scoping the inherited
/// store to `src`), so `<AssetsDir src="examples/wasm/assets" prefix="assets"/>`
/// serves `examples/wasm/assets/…` at `/assets/…`. `prefix` defaults to `src`.
///
/// One store, no env: the deployed app bucket is a replica of the checkout, so
/// `<AssetsDir src="assets"/>` resolves the same relative path either side of a
/// deploy.
#[template]
pub fn AssetsDir(
	/// The directory to mount, relative to the nearest ancestor store root.
	#[prop]
	src: RelPath,
	/// The url prefix to serve under; defaults to `src`.
	#[prop(default)]
	prefix: String,
	/// Browser cache TTL for the served files, forwarded to [`ServeBlobs`].
	cache: Option<Duration>,
) -> impl Bundle {
	let prefix = if prefix.is_empty() {
		src.to_string()
	} else {
		prefix
	};
	// the props struct directly rather than `rsx!`: an already-`Option` prop has no
	// call-site conversion, only the bare inner value does.
	(
		ServeBlobs { prefix, cache }.into_snippet_bundle(),
		DirPath(src),
	)
}

// the tests assemble a router with a temp store, both adjacent (on an ancestor) and
// co-located (on the route), to prove the self-or-ancestor lookup.
#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A temp store with a single `style.css` file.
	async fn css_store() -> BlobStore {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::from("style.css"), "body { color: red; }")
			.await
			.unwrap();
		store
	}

	/// A serve route mounted at `mount`, serving the captured path from a
	/// self-or-ancestor [`BlobStore`] (the [`ServeBlobs`] template expansion; the
	/// `DirPath`-scoped subdir case is covered by `bsx_site` end to end).
	fn serve_route(mount: &str) -> impl Bundle {
		ServeBlobs {
			prefix: mount.into(),
			cache: default(),
		}
		.into_snippet_bundle()
	}

	/// A store on an ancestor (the router) backs a child serve route: the
	/// composable pattern, no store built on the fly.
	#[beet_core::test]
	async fn serves_from_ancestor_store() {
		router_world()
			.spawn((Router::with_defaults(), css_store().await, children![
				serve_route("assets")
			]))
			.exchange(Request::get("assets/style.css"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("color: red");
	}

	/// A store co-located on the serve route entity also resolves (self is the
	/// nearest match), eg the `src`-seeded local store.
	#[beet_core::test]
	async fn serves_from_colocated_store() {
		router_world()
			.spawn((Router::with_defaults(), children![(
				serve_route("assets"),
				css_store().await
			)]))
			.exchange(Request::get("assets/style.css"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("color: red");
	}

	/// The real `site/` layout: a store rooted at the entry's dir plus
	/// `<AssetsDir src="assets"/>` serves a blog image out of `site/assets`, the
	/// one resolution dev and the deployed app bucket share. Skipped on a
	/// checkout without hydrated site assets (a fresh clone before
	/// `just site-shared pull`).
	#[beet_core::test]
	async fn site_assets_dir_serves_blog_image() {
		let Ok(site) = AbsPath::new_workspace_rel("site") else {
			return;
		};
		if !fs_ext::exists(site.join("assets/blog/kiama-sea-shanty-club.jpg"))
			.unwrap_or(false)
		{
			return;
		}
		router_world()
			.spawn((Router::with_defaults(), FsStore::new(site), children![
				AssetsDir {
					src: "assets".into(),
					prefix: default(),
					cache: default(),
				}
				.into_snippet_bundle()
			]))
			.exchange(Request::get("assets/blog/kiama-sea-shanty-club.jpg"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	/// A miss under `assets/` carries the unhydrated-checkout hint, and a miss
	/// anywhere else does not.
	#[beet_core::test]
	async fn miss_hints_at_an_unhydrated_assets_dir() {
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), BlobStore::temp(), children![
				serve_route("assets"),
				serve_route("other")
			]))
			.flush();
		world
			.entity_mut(root)
			.exchange(Request::get("assets/missing.png"))
			.await
			.text()
			.await
			.unwrap()
			.xpect_contains("may need a pull");
		world
			.entity_mut(root)
			.exchange(Request::get("other/missing.png"))
			.await
			.text()
			.await
			.unwrap()
			.xnot()
			.xpect_contains("may need a pull");
	}

	/// `?list` answers the keys under the requested path as json, relative to
	/// it, so an `HttpStore` on the mount lists the store it publishes.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn lists_keys_as_json() {
		let store = BlobStore::temp();
		for path in ["main.bsx", "docs/a.md", "docs/b/c.md"] {
			store.insert(&RelPath::from(path), "x").await.unwrap();
		}
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), store, children![serve_route(
				"repo"
			)]))
			.flush();
		let mut list = async |path: &str| -> Vec<RelPath> {
			world
				.entity_mut(root)
				.exchange(
					Request::get(path).with_param(HttpStore::LIST_PARAM, ""),
				)
				.await
				.json()
				.await
				.unwrap()
		};
		list("repo").await.xpect_eq(vec![
			RelPath::new("docs/a.md"),
			RelPath::new("docs/b/c.md"),
			RelPath::new("main.bsx"),
		]);
		list("repo/docs")
			.await
			.xpect_eq(vec![RelPath::new("a.md"), RelPath::new("b/c.md")]);
	}

	/// The mount read back through an [`HttpStore`] over a real listener: a
	/// key reads, a listing lists (the root and a subdir), and a missing key is
	/// absent rather than an error, the miss `SceneBlob` decides a first boot
	/// on.
	#[cfg(all(
		feature = "json",
		feature = "http",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn an_http_store_reads_the_mount() {
		let store = BlobStore::temp();
		for (path, body) in [
			("main.bsx", "<Router/>"),
			("docs/a.md", "# a"),
			("docs/b/c.md", "# c"),
		] {
			store.insert(&RelPath::from(path), body).await.unwrap();
		}
		let (mut server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		// leave the process-global loopback port to whoever owns it
		server.canonical = false;
		let url = server.local_url();
		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, on_spawn, store, children![(
					Router::default(),
					children![serve_route("repo")]
				)]))
				.run();
		});
		let http = BlobStore::new(HttpStore::new(format!("{url}/repo")));
		http.get_media(&RelPath::from("main.bsx"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("<Router/>");
		http.exists(&RelPath::from("docs/a.md"))
			.await
			.unwrap()
			.xpect_true();
		http.exists(&RelPath::from("missing.md"))
			.await
			.unwrap()
			.xpect_false();
		http.get(&RelPath::from("missing.md"))
			.await
			.unwrap_err()
			.downcast_ref::<HttpError>()
			.unwrap()
			.status_code
			.xpect_eq(StatusCode::NOT_FOUND);
		http.list().await.unwrap().xpect_eq(vec![
			RelPath::new("docs/a.md"),
			RelPath::new("docs/b/c.md"),
			RelPath::new("main.bsx"),
		]);
		http.with_subdir(RelPath::new("docs"))
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("a.md"), RelPath::new("b/c.md")]);
	}

	/// An extensionless path serves `<path>/index.html`, the static-host fallback.
	#[beet_core::test]
	async fn serves_index_html() {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::from("bar/index.html"), "<div>fallback</div>")
			.await
			.unwrap();
		router_world()
			.spawn((Router::with_defaults(), store, children![serve_route(
				"foo"
			)]))
			.exchange(Request::get("foo/bar"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
