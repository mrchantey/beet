//! Mounting a directory as a blob-store-backed static-file route.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A directory mounted as static-file routes, the markup-spawnable surface for
/// serving the binary assets a site references (favicon, images).
///
/// `<BlobStoreRoute src="assets"/>` roots a [`BlobStore`] at `src` (relative to
/// [`SiteRoot`]) and mounts it under [`Self::mount`] (defaulting to `src`), so it
/// serves `assets/branding/favicon-32x32.png` from `branding/favicon-32x32.png`
/// on disk. The trailing path is captured greedily; serving rules mirror a static
/// host (see [`serve_blob`]).
///
/// Unlike [`RoutesDir`] (which discovers content *pages*) this streams any file.
/// It is a [`template`](macro@template) rather than a marker component, so it
/// expands to its serve route at build time with no component left to re-fire on
/// reload.
#[template(system)]
pub fn BlobStoreRoute(
	/// The asset directory, relative to [`SiteRoot`].
	#[prop(into)]
	src: SmolPath,
	/// The url path the directory mounts at; defaults to `src`.
	#[prop]
	mount: Option<SmolPath>,
	site_root: Option<Res<SiteRoot>>,
) -> impl Bundle {
	let dir = site_root
		.map(|root| root.0.clone())
		.unwrap_or_else(|| SiteRoot::default().0)
		.join(&src);
	let mount = mount.unwrap_or(src);
	mount_blob_store(mount, BlobStore::new(FsStore::new(dir)))
}

/// Mount `store` at `mount`, serving files for any path beneath it.
///
/// The trailing path is captured greedily, so a `"assets"` mount serves
/// `assets/css/site.css` from `css/site.css` in the store. The single consumer
/// is [`BlobStoreRoute`]; the store is built at build time from a serializable
/// `src`, so the route itself stays declarative.
fn mount_blob_store(mount: SmolPath, store: BlobStore) -> impl Bundle {
	let mount = mount.as_str().trim_matches('/');
	let path = if mount.is_empty() {
		format!("*{STORE_PATH_PARAM}?")
	} else {
		format!("{mount}/*{STORE_PATH_PARAM}?")
	};
	(PathPartial::new(path), ServeStoreAction, store)
}

// the tests assemble a full `default_router` and a temp fs-backed store (both std).
#[cfg(all(test, feature = "std"))]
mod test {
	use super::mount_blob_store;
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn serves_file() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("style.css"), "body { color: red; }")
			.await
			.unwrap();
		router_world()
			.spawn((
				default_router(),
				children![mount_blob_store("assets".into(), store)],
			))
			.call::<Request, Response>(Request::get("assets/style.css"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("color: red");
	}

	#[beet_core::test]
	async fn serves_index_html() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("bar/index.html"), "<div>fallback</div>")
			.await
			.unwrap();
		router_world()
			.spawn((
				default_router(),
				children![mount_blob_store("foo".into(), store)],
			))
			.call::<Request, Response>(Request::get("foo/bar"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("<div>fallback</div>");
	}
}
