//! Request-scoped facts threaded into a render, read by scene systems.
//!
//! The layout middleware installs a [`RequestContext`] resource before the
//! document-shell scene is built and removes it after, so the shell's
//! `#[scene(system)]` widgets can read the current request and the matched
//! route entity via [`RenderQuery`]. The `#[scene(system)]` macro wires this
//! automatically for a `cx: &RequestContext` parameter.
//!
//! It is a resource rather than an ancestor component because scene systems run
//! during the build *before* their `ChildOf` edge is wired, so an ancestor walk
//! would not yet reach the shell root. Renders are synchronous (one context
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
/// [`RequestParts`] and the matched route entity. Installed as a resource for
/// the duration of a document-shell render.
#[derive(Debug, Clone, Resource)]
pub struct RequestContext {
	/// The parts of the request being rendered (method, url, headers, query).
	parts: RequestParts,
	/// The rendered content entity for this route, carrying any per-route
	/// components (eg [`ArticleMeta`](crate::prelude::ArticleMeta)) for widgets
	/// to query.
	route: Entity,
}

impl RequestContext {
	/// Build a context for the given request and matched route entity.
	pub fn new(parts: RequestParts, route: Entity) -> Self {
		Self { parts, route }
	}

	/// The parts of the request being rendered.
	pub fn parts(&self) -> &RequestParts { &self.parts }

	/// The rendered content entity, off which per-route components are queried.
	pub fn route(&self) -> Entity { self.route }

	/// The current request path as `/`-joined segments (no leading slash), eg
	/// `docs/intro`.
	pub fn current_path(&self) -> String { self.parts.path().join("/") }
}

/// Access to the active [`RequestContext`].
///
/// The `#[scene(system)]` macro injects this for a `cx: &RequestContext`
/// parameter, so widgets rarely name it directly. The `entity` argument of
/// [`get_context`](Self::get_context) is accepted for forward-compatibility
/// (per-subtree contexts) but currently unused.
#[derive(SystemParam)]
pub struct RenderQuery<'w> {
	context: Option<Res<'w, RequestContext>>,
}

impl RenderQuery<'_> {
	/// The active [`RequestContext`], erroring if none is installed (no render is
	/// in progress).
	pub fn get_context(&self, _entity: Entity) -> Result<&RequestContext> {
		self.context
			.as_deref()
			.ok_or_else(|| bevyhow!("no RequestContext: not inside a render"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	fn get_context_reads_resource() {
		let mut world = World::new();
		let route = world.spawn_empty().id();
		world.insert_resource(RequestContext::new(
			RequestParts::get("docs/intro"),
			route,
		));
		let entity = world.spawn_empty().id();
		world
			.run_system_cached_with(
				|entity: In<Entity>, query: RenderQuery| {
					query.get_context(*entity).unwrap().current_path()
				},
				entity,
			)
			.unwrap()
			.xpect_eq("docs/intro".to_string());
	}

	#[beet_core::test]
	fn get_context_errors_when_absent() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		world
			.run_system_cached_with(
				|entity: In<Entity>, query: RenderQuery| {
					query.get_context(*entity).is_err()
				},
				entity,
			)
			.unwrap()
			.xpect_true();
	}
}
