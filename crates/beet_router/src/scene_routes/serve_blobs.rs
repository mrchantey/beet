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
/// `prefix` is the only knob: `<ServeBlobs prefix="assets"/>` serves every file
/// beneath `assets/`. The greedy trailing capture and the request/response adapter
/// are private details ServeBlobs inserts for itself. Resolve the store by
/// composition: pair with a co-located store-scoping component (eg [`DirPath`] or
/// [`AssetsStore`]) on the same entity, or let it inherit an ancestor store.
#[template]
pub fn ServeBlobs(
	/// The mount path the static files are served under, eg `assets`.
	#[prop(into)]
	prefix: String,
) -> impl Bundle {
	(
		PathPartial::new(format!("{prefix}/*{STORE_PATH_PARAM}?")),
		ServeBlobsHandler,
		ExchangeOverload::new::<RequestParts, Response, _, _>(),
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
	serve_blob(&store, &path).await
}

/// Composes the asset-serving backing onto its route at build time: a dedicated
/// `BEET_ASSETS_BUCKET` [`S3Store`] when set (the deployed container's separate,
/// public-read assets bucket), else a [`DirPath`] scoping the nearest ancestor
/// site [`BlobStore`] to its `assets/` subdir (local dev). Spread alongside
/// [`ServeBlobs`], which owns the mount path:
/// `<ServeBlobs prefix="assets" {AssetsStore}/>`.
///
/// The assets glue, resolving where the served files come from; a WIP shim the
/// cli-rework will fold away.
#[template]
pub fn AssetsStore() -> impl Bundle {
	OnSpawn::new(|entity: &mut EntityWorldMut| {
		// the deployed assets bucket, mirroring `remote_site_store`'s endpoint /
		// region selection (R2 vs AWS S3). Only the native `aws_sdk` build can
		// build an `S3Store`; every other build scopes the ancestor site store.
		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		if let Ok(bucket) = env_ext::var("BEET_ASSETS_BUCKET") {
			let store = match env_ext::var("BEET_S3_ENDPOINT") {
				Ok(endpoint) => {
					S3Store::new(bucket, "auto").with_endpoint(endpoint)
				}
				Err(_) => S3Store::new(
					bucket,
					env_ext::var("AWS_REGION")
						.unwrap_or_else(|_| "us-west-2".to_string()),
				),
			};
			entity.insert(store);
			return;
		}
		// local dev: scope the inherited site store to its `assets/` subdir.
		entity.insert(DirPath("assets".into()));
	})
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
		ServeBlobs { prefix: mount.into() }.into_snippet_bundle()
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
