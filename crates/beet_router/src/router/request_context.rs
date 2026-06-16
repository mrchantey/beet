//! Request-scoped facts threaded into a render, read by layout scene systems.
//!
//! A render needs three handles it cannot reach by traversal: the matched
//! [`route`](RequestContext::route), the [`router`](RequestContext::router) that
//! owns its [`RouteTree`](crate::prelude::RouteTree), and the
//! [`content`](RequestContext::content) entity holding the rendered body. The
//! layout is built *detached* (the content is transcluded into a `<Slot>` by a
//! `Portal`, never reparented), so a layout widget has no `ChildOf` path back to
//! any of them. The layout middleware resolves them once and threads them here,
//! letting each widget do a direct read instead of re-deriving facts by a walk.
//!
//! These facts live on a [`RequestContextStack`] resource, not an ancestor
//! component, because scene systems run during the build *before* their
//! `ChildOf` edge is wired, so an ancestor walk would not yet reach the layout
//! root. A *stack* (rather than a lone resource) keeps renders re-entrant: a
//! render may synchronously dispatch a nested render, which pushes its own
//! context and pops it on completion, leaving the outer context intact on top.
//!
//! Per-route metadata is **not** carried here: it lives on the route/content
//! entity (eg [`ArticleMeta`](crate::prelude::ArticleMeta) from frontmatter).
//! Widgets query whatever components they need off
//! [`content`](RequestContext::content), keeping this fixed type decoupled from
//! the user-extensible metadata mapped from frontmatter.
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A re-entrant stack of [`RequestContext`]s: the active render's context is the
/// top, pushed when a layout render begins and popped when it ends.
///
/// Installed as a resource by [`RouterPlugin`](crate::prelude::RouterPlugin).
/// Layout `#[template(system)]` widgets read the active context with
/// [`current`](Self::current) through a `Res<RequestContextStack>` parameter.
#[derive(Debug, Default, Clone, Resource)]
pub struct RequestContextStack(Vec<RequestContext>);

impl RequestContextStack {
	/// Push `cx` as the active context for the duration of a render.
	pub fn push(&mut self, cx: RequestContext) { self.0.push(cx); }

	/// Pop the active context when its render completes, restoring any outer
	/// render's context as the new top.
	pub fn pop(&mut self) -> Option<RequestContext> { self.0.pop() }

	/// The active (top) context.
	///
	/// # Panics
	///
	/// Panics if read with an empty stack, ie outside a render. A layout widget
	/// never is: the middleware pushes the context before building the layout.
	pub fn current(&self) -> &RequestContext {
		self.0
			.last()
			.expect("no active RequestContext: read outside a layout render")
	}
}

/// Request-scoped facts available to layout scene systems during a render. Held
/// on the [`RequestContextStack`]; see its docs for the re-entrancy model.
#[derive(Debug, Clone)]
pub struct RequestContext {
	/// The parts of the request being rendered (method, url, headers, query).
	parts: RequestParts,
	/// The rendered content entity for this route, carrying any per-route
	/// components (eg [`ArticleMeta`](crate::prelude::ArticleMeta)) for widgets
	/// to query. This may be a *detached* render root (a per-request page built
	/// by `spawn_template`), so it is not a reliable route-tree anchor.
	content: Entity,
	/// The matched route entity in the [`RouteTree`](crate::prelude::RouteTree)
	/// (the action the router dispatched to). Always lives in the served tree
	/// (unlike [`content`](Self::content), which may be detached), so it is the
	/// in-tree anchor the [`router`](Self::router) is resolved from at build time.
	route: Entity,
	/// The entity owning this request's [`RouteTree`](crate::prelude::RouteTree),
	/// resolved once as the nearest tree-bearing ancestor of [`route`](Self::route).
	/// Tree-scoped widgets (eg [`RouteSidebar`](crate::prelude::RouteSidebar))
	/// read the tree directly off this handle, never re-deriving it by an ancestor
	/// walk, so the nav is scoped to *this* request's tree even when other
	/// [`RouteTree`]s share the world.
	router: Entity,
}

impl RequestContext {
	/// Build a context for the given request, rendered content entity, matched
	/// route entity, and the [`router`](Self::router) owning its route tree.
	pub fn new(
		parts: RequestParts,
		content: Entity,
		route: Entity,
		router: Entity,
	) -> Self {
		Self {
			parts,
			content,
			route,
			router,
		}
	}

	/// The parts of the request being rendered.
	pub fn parts(&self) -> &RequestParts { &self.parts }

	/// The rendered content entity, off which per-route components are queried.
	/// May be detached from the route tree (a per-request render root).
	pub fn content(&self) -> Entity { self.content }

	/// The matched route entity in the served tree, the in-tree anchor the
	/// [`router`](Self::router) handle was resolved from.
	pub fn route(&self) -> Entity { self.route }

	/// The entity owning this request's [`RouteTree`](crate::prelude::RouteTree).
	/// A tree-scoped widget reads the tree directly off this handle (an O(1)
	/// component get, no ancestor walk).
	pub fn router(&self) -> Entity { self.router }

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
	fn current_reads_top_of_stack() {
		// scene systems read the active context off the top of the stack.
		let mut world = World::new();
		let route = world.spawn_empty().id();
		let mut stack = RequestContextStack::default();
		stack.push(RequestContext::new(
			RequestParts::get("docs/intro"),
			route,
			route,
			route,
		));
		world.insert_resource(stack);
		world
			.run_system_cached(|stack: Res<RequestContextStack>| {
				stack.current().current_path()
			})
			.unwrap()
			.xpect_eq("docs/intro".to_string());
	}

	#[beet_core::test]
	fn stack_is_re_entrant() {
		// a nested render pushes its own context and pops it on completion,
		// leaving the outer render's context intact on top.
		let mut world = World::new();
		let outer = world.spawn_empty().id();
		let inner = world.spawn_empty().id();
		let mut stack = RequestContextStack::default();
		stack.push(RequestContext::new(
			RequestParts::get("outer"),
			outer,
			outer,
			outer,
		));
		stack.push(RequestContext::new(
			RequestParts::get("inner"),
			inner,
			inner,
			inner,
		));
		stack.current().route().xpect_eq(inner);
		stack.pop();
		stack.current().route().xpect_eq(outer);
	}
}
