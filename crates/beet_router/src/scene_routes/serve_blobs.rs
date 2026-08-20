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
#[template]
pub fn ServeBlobs(
	/// The mount path the static files are served under, eg `assets`.
	#[prop(into)]
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
#[action(handler_only)]
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
		.map(|segments| SmolPath::from_segments(segments))
		.unwrap_or_else(|| SmolPath::from(cx.input.path()));
	serve_blob(&store, &path)
		.await
		.map_err(|err| unhydrated_hint(&cx.input, err))
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
		.filter(|_| request.path().first().map(SmolStr::as_str) == Some(HINT_DIR))
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
	#[prop(into)]
	src: String,
	/// The url prefix to serve under; defaults to `src`.
	#[prop(into, default)]
	prefix: String,
	/// Browser cache TTL for the served files, forwarded to [`ServeBlobs`].
	cache: Option<Duration>,
) -> impl Bundle {
	let prefix = if prefix.is_empty() {
		src.clone()
	} else {
		prefix
	};
	// the props struct directly rather than `rsx!`: an already-`Option` prop has no
	// call-site conversion, only the bare inner value does.
	(
		ServeBlobs {
			prefix,
			cache: PropOpt(cache),
		}
		.into_snippet_bundle(),
		DirPath(SmolPath::from(src.as_str())),
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
			.insert(&SmolPath::from("style.css"), "body { color: red; }")
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
		let Ok(site) = AbsPathBuf::new_workspace_rel("site") else {
			return;
		};
		if !site.join("assets/blog/kiama-sea-shanty-club.jpg").exists() {
			return;
		}
		router_world()
			.spawn((
				Router::with_defaults(),
				FsStore::new(site),
				children![
					AssetsDir {
						src: "assets".into(),
						prefix: default(),
						cache: default(),
					}
					.into_snippet_bundle()
				],
			))
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
			.spawn((
				Router::with_defaults(),
				BlobStore::temp(),
				children![serve_route("assets"), serve_route("other")],
			))
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

	/// An extensionless path serves `<path>/index.html`, the static-host fallback.
	#[beet_core::test]
	async fn serves_index_html() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("bar/index.html"), "<div>fallback</div>")
			.await
			.unwrap();
		router_world()
			.spawn((Router::with_defaults(), store, children![serve_route("foo")]))
			.exchange(Request::get("foo/bar"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
