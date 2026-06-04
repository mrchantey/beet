//! Serving static files from a [`BlobStore`] as routes.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment name used by [`serve_store`] to capture the file path.
const STORE_PATH_PARAM: &str = "store_path";

/// Mounts a [`BlobStore`] at `mount`, serving files for any path beneath it.
///
/// The trailing path is captured greedily, so `serve_store("assets", store)`
/// serves `assets/css/site.css` from `css/site.css` in the store. Serving rules
/// mirror a static host (see [`serve_blob`]): extensioned paths redirect to a
/// public URL when the store has one, otherwise stream the bytes; extensionless
/// paths resolve to `<path>/index.html`.
pub fn serve_store(mount: impl AsRef<str>, store: BlobStore) -> impl Bundle {
	let mount = mount.as_ref().trim_matches('/');
	let path = if mount.is_empty() {
		format!("*{STORE_PATH_PARAM}?")
	} else {
		format!("{mount}/*{STORE_PATH_PARAM}?")
	};
	(PathPartial::new(path), ServeStoreAction, store)
}

/// Serves the captured path from the ancestor [`BlobStore`].
#[action(route, handler_only)]
#[derive(Default, Component)]
async fn ServeStoreAction(cx: ActionContext<RequestParts>) -> Result<Response> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
			|entity, query| query.get(entity).cloned(),
		)
		.await??;

	// the greedy capture is the file path relative to the mount point
	let path = cx
		.input
		.get_params(STORE_PATH_PARAM)
		.map(|segments| SmolPath::from_segments(segments))
		.unwrap_or_else(|| SmolPath::from(cx.input.path()));

	serve_blob(&store, &path).await
}

/// Serves a single path from a [`BlobStore`] using static-host conventions:
/// - extensioned path + a store public URL → permanent redirect
/// - extensioned path, no public URL → stream the bytes (mime from extension)
/// - extensionless path → serve `<path>/index.html` as HTML
pub async fn serve_blob(
	store: &BlobStore,
	path: &SmolPath,
) -> Result<Response> {
	if path.extension().is_some() {
		if let Some(url) = store.public_url(path).await? {
			Response::permanent_redirect(url).xok()
		} else {
			store
				.get_media(path)
				.await
				.map(|media| Response::ok().with_media(media))
		}
	} else {
		store
			.get_media(&path.join("index.html"))
			.await
			.map(|media| Response::ok().with_media(media))
	}
}


// the tests assemble a full `default_router` and a temp fs-backed store (both std).
#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn serve_blob_streams_file() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("style.css"), "body { color: red; }")
			.await
			.unwrap();
		super::serve_blob(&store, &SmolPath::from("style.css"))
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq("body { color: red; }");
	}

	#[beet_core::test]
	async fn serve_blob_appends_index_html() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("docs/index.html"), "<h1>Hello</h1>")
			.await
			.unwrap();
		super::serve_blob(&store, &SmolPath::from("docs"))
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq("<h1>Hello</h1>");
	}

	#[beet_core::test]
	async fn serve_store_route_serves_file() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("style.css"), "body { color: red; }")
			.await
			.unwrap();
		router_world()
			.spawn((default_router(), children![serve_store("assets", store)]))
			.call::<Request, Response>(Request::get("assets/style.css"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("color: red");
	}

	#[beet_core::test]
	async fn serve_store_route_serves_index_html() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("bar/index.html"), "<div>fallback</div>")
			.await
			.unwrap();
		router_world()
			.spawn((default_router(), children![serve_store("foo", store)]))
			.call::<Request, Response>(Request::get("foo/bar"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
