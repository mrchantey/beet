//! Mounting files from a [`BlobStore`] as static-file routes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment name [`BlobStoreRoute`] captures the trailing file path into.
pub(crate) const STORE_PATH_PARAM: &str = "store_path";

/// A directory of files served as static-file routes, the markup-spawnable surface
/// for the binary assets a site references (favicon, images).
///
/// `<BlobStoreRoute mount="assets"/>` mounts a serve route under `mount` that
/// streams any file beneath it from the nearest self-or-ancestor [`BlobStore`], so
/// an *adjacent* store (an [`FsStore`] in dev, an S3 store in production) backs the
/// route, composed rather than created on the fly. The trailing path is captured
/// greedily; serving rules mirror a static host (see [`serve_blob`]).
///
/// A no-code site has no Rust seam to compose a store, so the optional `src` seeds
/// a local [`FsStore`] on the route itself: `<BlobStoreRoute src="assets"/>` roots
/// a store at `src` (relative to [`SiteRoot`]) and serves it at `assets/…`. Provide
/// `src` to mount a directory, or omit it to serve from an adjacent store.
///
/// Unlike [`RoutesDir`] (which discovers content *pages*) this streams any file. It
/// is a [`template`](macro@template) rather than a marker component, so it expands
/// to its serve route at build time with no component left to re-fire on reload.
#[template(system)]
pub fn BlobStoreRoute(
	/// The url path the files mount at; defaults to `src`, else the root.
	#[prop(into)]
	mount: Option<SmolPath>,
	/// An optional directory (relative to [`SiteRoot`]) seeding a local [`FsStore`]
	/// on this route; omit to serve from an adjacent [`BlobStore`].
	#[prop(into)]
	src: Option<SmolPath>,
	site_root: Option<Res<SiteRoot>>,
) -> impl Bundle {
	// the mount defaults to `src` (so `src="assets"` serves at `assets/…`), else the
	// root; the trailing file path is captured greedily.
	let mount = mount.or_else(|| src.clone()).unwrap_or_default();
	let mount = mount.as_str().trim_matches('/');
	let path = if mount.is_empty() {
		format!("*{STORE_PATH_PARAM}?")
	} else {
		format!("{mount}/*{STORE_PATH_PARAM}?")
	};
	// `src` seeds a local FsStore on the route; otherwise the serve action resolves
	// the nearest ancestor store (composed by the caller).
	let store = src.map(|src| {
		let dir = site_root
			.map(|root| root.0.clone())
			.unwrap_or_else(|| SiteRoot::default().0)
			.join(&src);
		BlobStore::new(FsStore::new(dir))
	});
	(
		PathPartial::new(path),
		ServeStoreAction,
		OnSpawn::insert_option(store),
	)
}

/// Serves the captured path from the nearest self-or-ancestor [`BlobStore`], the
/// handler [`BlobStoreRoute`] expands to.
///
/// Resolves the store by composition rather than constructing one, so any adjacent
/// store (filesystem, S3, in-memory) backs the route. The greedy capture is the
/// file path relative to the mount point.
#[action(route, handler_only)]
#[derive(Default, Component)]
pub(crate) async fn ServeStoreAction(
	cx: ActionContext<RequestParts>,
) -> Result<Response> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
			|entity, query| query.get(entity).cloned(),
		)
		.await??;
	let path = cx
		.input
		.get_params(STORE_PATH_PARAM)
		.map(|segments| SmolPath::from_segments(segments))
		.unwrap_or_else(|| SmolPath::from(cx.input.path()));
	serve_blob(&store, &path).await
}

// the tests assemble a router with a temp fs-backed store, both adjacent (on an
// ancestor) and co-located (on the route), to prove the self-or-ancestor lookup.
#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
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
	/// self-or-ancestor [`BlobStore`] (the [`BlobStoreRoute`] expansion, minus the
	/// build-time `src` FsStore seed which the `bsx_site` test covers end to end).
	fn serve_route(mount: &str) -> impl Bundle {
		(
			PathPartial::new(format!("{mount}/*{STORE_PATH_PARAM}?")),
			ServeStoreAction,
		)
	}

	/// A store on an ancestor (the router) backs a child serve route: the
	/// composable pattern, no store built on the fly.
	#[beet_core::test]
	async fn serves_from_ancestor_store() {
		router_world()
			.spawn((default_router(), css_store().await, children![
				serve_route("assets")
			]))
			.call::<Request, Response>(Request::get("assets/style.css"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("color: red");
	}

	/// A store co-located on the serve route entity also resolves (self is the
	/// nearest match), eg the `src`-seeded local store.
	#[beet_core::test]
	async fn serves_from_colocated_store() {
		router_world()
			.spawn((default_router(), children![(
				serve_route("assets"),
				css_store().await
			)]))
			.call::<Request, Response>(Request::get("assets/style.css"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("color: red");
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
			.spawn((default_router(), store, children![serve_route("foo")]))
			.call::<Request, Response>(Request::get("foo/bar"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
