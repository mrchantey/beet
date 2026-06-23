//! Serving files from a [`BlobStore`] as static-file routes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment name [`ServeBlobs`] captures the trailing file path into.
pub(crate) const STORE_PATH_PARAM: &str = "store_path";

/// A markup-spawnable route whose `path` becomes a [`PathPartial`].
///
/// The url and the behavior are separate concerns, both declared at the call site:
/// the `path` prop is the route pattern, and the handler (plus any store scoping)
/// rides a component spread on the same entity. The static-asset mount is
/// `<Route path="assets/*store_path?" {(ServeBlobs, DirPath("assets"))}/>`: the
/// greedy [`STORE_PATH_PARAM`] capture streams any file beneath `assets/`, and the
/// [`DirPath`] scopes the resolved store to the site store's `assets/` subdir.
///
/// The Rust equivalent is the [`route`](crate::prelude::route) helper, which this
/// expands to. It is a [`template`](macro@template) rather than a marker component,
/// so it expands to a [`PathPartial`] at build time with no component left to
/// re-fire on reload.
#[template]
pub fn Route(
	/// The route path pattern, eg `assets/*store_path?`; defaults to the root.
	#[prop(into)]
	path: String,
) -> impl Bundle {
	PathPartial::new(path)
}

/// Serves the captured path from the nearest self-or-ancestor [`BlobStore`].
///
/// Resolves the store by composition rather than constructing one, so any adjacent
/// store (filesystem, S3, in-memory) backs the route. A co-located [`DirPath`] has
/// already scoped that store to its subdir, so this just reads the resolved store.
/// The greedy [`STORE_PATH_PARAM`] capture is the file path relative to the mount
/// point; serving rules mirror a static host (see [`serve_blob`]). Pair it with a
/// [`Route`] carrying the `*store_path?` capture (and optionally a [`DirPath`]).
#[action(route, handler_only)]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn ServeBlobs(cx: ActionContext<RequestParts>) -> Result<Response> {
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
	/// self-or-ancestor [`BlobStore`] (the [`Route`] + [`ServeBlobs`] expansion; the
	/// `DirPath`-scoped subdir case is covered by `bsx_site` end to end).
	fn serve_route(mount: &str) -> impl Bundle {
		route(&format!("{mount}/*{STORE_PATH_PARAM}?"), ServeBlobs)
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
