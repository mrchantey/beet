//! Scene routes — routes that return a renderable entity tree (a scene) rather
//! than serialized data.
//!
//! A regular `exchange_route` returns an `IntoResponseWithRequestParts` (JSON, a redirect,
//! bytes) already in final form. A *scene route* instead yields the [`Entity`]
//! root of a tree (an rsx/markdown/parsed document, a behavior tree, …)
//! carrying a [`PageRoot`]; a `NodeRenderer` walks [`PageRoot::rendered`]
//! and serializes it per the request's `Accept` header (HTML, markdown,
//! charcell, …), then despawns the [`DespawnAfterRender`] entities.
//!
//! Handlers produce a content [`Bundle`]; the [`render_action`] constructors
//! build a complete route from a path + handler:
//! [`render_action::fixed_func_route`] (static, per request), and
//! [`render_action::pure_route`] / [`render_action::async_route`] /
//! [`render_action::system_route`] (per handler kind). The tree is serialized
//! by [`default_renderer`]. [`FixedPage`] is the persistent exception: its route
//! entity *is* the render root, so one live tree serves every request.

mod page_root;
pub use page_root::*;
// the persistent counterpart of the per-request page routes: one live tree,
// served request after request.
mod fixed_page;
pub use fixed_page::*;
mod default_renderer;
pub mod render_action;
pub use default_renderer::*;
mod route_query;
pub use route_query::*;
// reactive template registration: a `<TemplateDir src="templates"/>` reads its
// dir through the store its derived `DirPath` scoped and registers each
// template, like `RoutesDir`.
#[cfg(feature = "bsx")]
mod template_dir;
#[cfg(feature = "bsx")]
pub use template_dir::*;
// the entry-declared store root (`<RepoRoot src="../.."/>`), pre-scanned by
// entry resolution like an entry's own `<TemplateDir>`s.
#[cfg(feature = "bsx")]
mod repo_root;
#[cfg(feature = "bsx")]
pub use repo_root::*;
// the registered set of declarations that act before the entry builds, and the
// one registry-free walk entry resolution reads them (and `<Template src>`) from.
#[cfg(feature = "bsx")]
mod prescan;
#[cfg(feature = "bsx")]
pub use prescan::*;
// `RoutesDir` + its discovery is compiled on every std target: one observer
// scans the store off the async runtime, so native and wasm share the path.
mod routes_dir;
pub use routes_dir::*;
// the `ServeBlobs` static-file route: it owns its mount prefix and serves from the
// nearest `BlobStore`. cross-platform: the wasm Worker serves a site's assets too.
mod serve_blobs;
pub use serve_blobs::*;

use beet_core::prelude::*;
use beet_net::prelude::*;

/// The systems reacting to a changed [`Blob`] or [`BlobStore`] under a
/// [`RoutesDir`]/[`TemplateDir`]: a route or template file re-read, a dir
/// rescan. A reload dispatcher orders after this set, so the pending guards
/// the reactions park are seen before it decides the subtree has settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct BlobReactions;

/// Rescan each `D` dir whose scoped store changed, ie a file was created or
/// removed under it: re-inserting the dir re-fires its scan (a swap for a
/// [`RoutesDir`], a child diff for a [`TemplateDir`]). The store's own arrival
/// is skipped, the dir's insert having fired the first scan.
pub(crate) fn rescan_changed_dirs<D: Component + Clone>(
	dirs: Query<(Entity, &D, Ref<BlobStore>), Changed<BlobStore>>,
	mut commands: Commands,
) {
	for (entity, dir, _) in
		dirs.iter().filter(|(_, _, store)| !store.is_added())
	{
		commands.entity(entity).insert(dir.clone());
	}
}

/// The scoped [`BlobStore`] a dir's derived [`DirPath`] produced on `entity`,
/// the store its async scan reads through; an error naming the tag when no
/// ancestor store backs it (the dir must be a child of its store's entity).
pub(crate) fn scoped_store(
	stores: &Query<&BlobStore>,
	entity: Entity,
	tag: &str,
	src: &RelPath,
) -> Result<BlobStore> {
	stores.get(entity).cloned().ok().ok_or_else(|| {
		bevyhow!(
			"`<{tag} src=\"{src}\">` has no store: it scopes the nearest \
			ancestor `BlobStore`, which is missing"
		)
	})
}
