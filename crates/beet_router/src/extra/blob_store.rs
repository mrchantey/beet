//! Serving a path from a [`BlobStore`] with static-host conventions, the shared
//! primitive behind [`ServeBlobs`](crate::prelude::ServeBlobs) and the
//! [`HtmlStore`](crate::prelude::HtmlStore) gate.

use beet_core::prelude::*;
use beet_net::prelude::*;

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
