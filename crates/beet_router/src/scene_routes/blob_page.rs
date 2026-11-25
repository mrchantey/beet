//! A store file served as a page: [`BlobPage`] parses the file's bytes into a
//! fresh tree per request, its root carrying the components the document
//! declares there, so a widget reading the render root finds the page's
//! metadata whether or not any scan discovered the file.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use beet_ui::prelude::*;

/// Serves the file at `path` in the nearest ancestor [`BlobStore`], parsed
/// into a render tree per request. The file is the route's [`Blob`], derived
/// from `path` like any [`BlobPath`], so an edit to it marks the route changed
/// (see [`refresh_changed_routes`]).
#[action(route)]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::component_hook(|page: &BlobPage| BlobPath::derive(&page.path)))]
pub async fn BlobPage(
	/// The store-relative path of the file this route serves.
	#[field]
	path: RelPath,
	cx: ActionContext<Request>,
) -> Result<PageRequest> {
	// the route's blob, resolved from the nearest ancestor store; absent is an
	// error, never an implicit filesystem store (there is none on wasm).
	let blob = cx
		.caller
		.get::<Blob, _>(Blob::clone)
		.await
		.ok()
		.ok_or_else(|| {
			bevyhow!(
				"`{path}` has no store: `BlobPage` reads through the nearest \
				 ancestor `BlobStore`, which is missing"
			)
		})?;
	BlobPage::serve(&cx.caller, cx.input.parts().clone(), blob).await
}

impl BlobPage {
	/// Serve the store file at `path`.
	pub fn new(path: impl Into<RelPath>) -> Self { Self { path: path.into() } }

	/// Parses `blob` into a fresh render root for the request `parts` to
	/// `caller`, the route serving it.
	pub(crate) async fn serve(
		caller: &AsyncEntity,
		parts: RequestParts,
		blob: Blob,
	) -> Result<PageRequest> {
		// the in-tree route anchor and the request being served, threaded into the
		// render context the content builds under (below)
		let route = caller.id();
		let bytes = blob.get_media().await?;
		// the package the page's references resolve in, a detached tree carrying
		// its own, and the component its unsectioned metadata declares
		let (package, frontmatter_type) = caller
			.with_state::<(PackageQuery, AncestorQuery<&FrontmatterType>), _>(
				|entity, (packages, types)| {
					packages.get(entity).map(|package| {
						let frontmatter_type = types
							.get(entity)
							.cloned()
							.unwrap_or_default()
							.component;
						(package, frontmatter_type)
					})
				},
			)
			.await??;
		let path = package.path_of(&blob);
		let template = caller
			.world()
			.with({
				let path = path.clone();
				move |world: &mut World| -> Result<DynamicTemplate> {
					DocParsers::of(world)
						.with_metadata_type(frontmatter_type)
						.body(&bytes)?
						.reflect_at(world, &path)
				}
			})
			.await?;
		// the documents it instantiates are loaded before it builds, so they
		// build with it, under the request's context
		PrefabCache::load_instances(
			&caller.world(),
			&package,
			&path,
			&template,
		)
		.await?;
		// carry the store onto the render root: the per-request tree is a detached root
		// (below), so a render-time widget that reads a file (eg `<CodeSnippet src>`)
		// resolves it by self-or-ancestor lookup rather than walking to the router.
		let render_store = blob.store().clone();

		// parse into a fresh entity per request, never the route node itself. The route
		// node is persistent and shared: it serves http alongside many live TUI surfaces
		// (one per SSH session), and the live path keeps each rendered tree alive. Parsing
		// into the node would make every surface transclude the one shared content, so a
		// second session navigating here doubles the body on every surface bound to it.
		// The fresh tree is a self-referential render root, ephemeral and despawned after
		// render, exactly like the per-request `spawn_render_step` content.
		caller
			.world()
			.with(move |world: &mut World| -> Result<PageRequest> {
				let content = world
					.spawn((render_store, PackageStore::new(package)))
					.id();
				// the entity owning this request's route tree, resolved from the
				// in-tree route exactly as the layout middleware resolves it
				let router = world
					.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
						trees.get_entity(route)
					})
					.unwrap_or(route);
				// the page is a request-scoped render like the layout around it, so
				// its own build gets the same context: that is what lets a route-aware
				// widget (`<RouteIndex/>`) live in the CONTENT rather than only in a
				// layout, where it would have no request to read.
				RequestContextStack::scoped(
					world,
					RequestContext::new(parts, content, route, router),
					|world| -> Result<PageRequest> {
						let mut entity = world.entity_mut(content);
						DocParsers::build(&mut entity, template)?;
						PageRoot::insert(&mut entity, vec![content]);
						Ok(PageRequest(content))
					},
				)
			})
			.await
	}
}
