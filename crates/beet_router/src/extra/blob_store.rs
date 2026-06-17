//! Serving static files from a [`BlobStore`] as routes.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment name used by [`BlobStoreRoute`] to capture the file path.
pub(crate) const STORE_PATH_PARAM: &str = "store_path";

/// Serves the captured path from the ancestor [`BlobStore`].
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


// the tests serve from a temp fs-backed store (std-only).
#[cfg(all(test, feature = "std"))]
mod test {
	use beet_core::prelude::*;
	use beet_net::prelude::*;

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
}
