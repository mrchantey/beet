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
/// To serve a *subdirectory* of the site store (the one-bucket layout, site at the
/// root and assets under `assets/`), pair it with a [`DirPath`] on the same entity:
/// `<BlobStoreRoute mount="assets" {DirPath("assets")}/>` scopes the resolved store
/// to `assets/`. The `mount` (url prefix) and the [`DirPath`] (store subdir) are
/// separate concerns, so the two-bucket layout (an assets store composed directly
/// above, no subdir) just omits the [`DirPath`].
///
/// Unlike [`RoutesDir`] (which discovers content *pages*) this streams any file. It
/// is a [`template`](macro@template) rather than a marker component, so it expands
/// to its serve route at build time with no component left to re-fire on reload.
#[template]
pub fn BlobStoreRoute(
	/// The url path the files mount at; defaults to the root.
	#[prop(into)]
	mount: Option<SmolPath>,
) -> impl Bundle {
	// the trailing file path is captured greedily under the mount point.
	let mount = mount.unwrap_or_default();
	let mount = mount.as_str().trim_matches('/');
	let path = if mount.is_empty() {
		format!("*{STORE_PATH_PARAM}?")
	} else {
		format!("{mount}/*{STORE_PATH_PARAM}?")
	};
	(PathPartial::new(path), ServeStoreAction)
}

/// Serves the captured path from the nearest self-or-ancestor [`BlobStore`], the
/// handler [`BlobStoreRoute`] expands to.
///
/// Resolves the store by composition rather than constructing one, so any adjacent
/// store (filesystem, S3, in-memory) backs the route. A co-located [`DirPath`] has
/// already scoped that store to its subdir, so this just reads the resolved store.
/// The greedy capture is the file path relative to the mount point.
#[action(route, handler_only)]
#[derive(Default, Component)]
pub(crate) async fn ServeStoreAction(
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
	serve_blob(&store, &path).await
}

// the tests assemble a router with a temp fs-backed store, both adjacent (on an
// ancestor) and co-located (on the route), to prove the self-or-ancestor lookup.
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
	/// self-or-ancestor [`BlobStore`] (the [`BlobStoreRoute`] expansion; the
	/// `DirPath`-scoped subdir case is covered by `bsx_site` end to end).
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
			.spawn((default_router(), children![(
				serve_route("assets"),
				css_store().await
			)]))
			.exchange(Request::get("assets/style.css"))
			.await
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
			.exchange(Request::get("foo/bar"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
