//! Request-scoped facts threaded into a render, read by scene systems.
//!
//! The layout middleware installs a [`RequestContext`] resource before the
//! document-layout scene is built and removes it after, so the layout's
//! `#[template(system)]` widgets can read the current request and the matched
//! route entity through an ordinary `cx: Res<RequestContext>` parameter.
//!
//! It is a resource rather than an ancestor component because scene systems run
//! during the build *before* their `ChildOf` edge is wired, so an ancestor walk
//! would not yet reach the layout root. Renders are synchronous (one context
//! active at a time), so a resource is sound.
//!
//! Per-route metadata is **not** carried here: it lives on the route entity
//! (eg [`ArticleMeta`](crate::prelude::ArticleMeta) from frontmatter). Widgets
//! query whatever components they need off [`route`](RequestContext::route),
//! keeping this fixed type decoupled from the user-extensible metadata mapped
//! from frontmatter.
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Request-scoped facts available to scene systems during a render: the
/// [`RequestParts`], the rendered content entity, and the matched route entity.
/// Installed as a resource for the duration of a document-layout render.
#[derive(Debug, Clone, Resource)]
pub struct RequestContext {
	/// The parts of the request being rendered (method, url, headers, query).
	parts: RequestParts,
	/// The rendered content entity for this route, carrying any per-route
	/// components (eg [`ArticleMeta`](crate::prelude::ArticleMeta)) for widgets
	/// to query. This may be a *detached* render root (a per-request page built
	/// by `spawn_template`), so it is not a reliable route-tree anchor.
	content: Entity,
	/// The matched route entity in the [`RouteTree`](crate::prelude::RouteTree)
	/// (the action the router dispatched to). Unlike [`content`](Self::content)
	/// this always lives in the served tree, so widgets that must scope to *their*
	/// tree (eg [`RouteSidebar`](crate::prelude::RouteSidebar)) anchor an ancestor
	/// walk here.
	route: Entity,
}

impl RequestContext {
	/// Build a context for the given request, rendered content entity, and the
	/// matched route entity.
	pub fn new(parts: RequestParts, content: Entity, route: Entity) -> Self {
		Self {
			parts,
			content,
			route,
		}
	}

	/// The parts of the request being rendered.
	pub fn parts(&self) -> &RequestParts { &self.parts }

	/// The rendered content entity, off which per-route components are queried.
	/// May be detached from the route tree (a per-request render root), so use
	/// [`route`](Self::route) to anchor a tree walk.
	pub fn content(&self) -> Entity { self.content }

	/// The matched route entity in the served tree, the reliable anchor for
	/// route-tree resolution (it is always a tree descendant, where the rendered
	/// [`content`](Self::content) may be detached).
	pub fn route(&self) -> Entity { self.route }

	/// The current request path as `/`-joined segments (no leading slash), eg
	/// `docs/intro`.
	pub fn current_path(&self) -> String { self.parts.path().join("/") }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	fn read_context_resource() {
		// scene systems read the installed context through `Res<RequestContext>`.
		let mut world = World::new();
		let route = world.spawn_empty().id();
		world.insert_resource(RequestContext::new(
			RequestParts::get("docs/intro"),
			route,
			route,
		));
		world
			.run_system_cached(|cx: Res<RequestContext>| cx.current_path())
			.unwrap()
			.xpect_eq("docs/intro".to_string());
	}
}
