//! Request-scoped facts threaded into a render, read by scene systems.
//!
//! The layout middleware installs a [`RouteContext`] resource before the
//! document-shell scene is built and removes it after, so the shell's
//! `#[scene(system)]` widgets can read the current path and the matched route's
//! metadata via [`RenderQuery`]. The `#[scene(system)]` macro wires this
//! automatically for a `cx: &RouteContext` parameter.
//!
//! It is a resource rather than an ancestor component because scene systems run
//! during the build *before* their `ChildOf` edge is wired, so an ancestor walk
//! would not yet reach the shell root. Renders are synchronous (one context
//! active at a time), so a resource is sound.
use crate::prelude::*;
use beet_core::prelude::*;

/// Request-scoped facts available to scene systems during a render: the current
/// path and the matched route's [`ArticleMeta`]. Installed as a resource for the
/// duration of a document-shell render.
#[derive(Debug, Default, Clone, Resource)]
pub struct RouteContext {
	/// The request path being rendered, as `/`-joined segments (no leading
	/// slash), eg `docs/intro`.
	path: String,
	/// Per-page metadata for the matched route (from markdown frontmatter).
	article_meta: ArticleMeta,
}

impl RouteContext {
	/// Build a context for the given path and matched-route metadata.
	pub fn new(path: impl Into<String>, article_meta: ArticleMeta) -> Self {
		Self {
			path: path.into(),
			article_meta,
		}
	}

	/// The current request path (`/`-joined segments, no leading slash).
	pub fn current_path(&self) -> &str { &self.path }

	/// The matched route's metadata (title/description/sidebar).
	pub fn article_meta(&self) -> &ArticleMeta { &self.article_meta }
}

/// Access to the active [`RouteContext`].
///
/// The `#[scene(system)]` macro injects this for a `cx: &RouteContext`
/// parameter, so widgets rarely name it directly. The `entity` argument of
/// [`get_context`](Self::get_context) is accepted for forward-compatibility
/// (per-subtree contexts) but currently unused.
#[derive(SystemParam)]
pub struct RenderQuery<'w> {
	context: Option<Res<'w, RouteContext>>,
}

impl RenderQuery<'_> {
	/// The active [`RouteContext`], erroring if none is installed (no render is
	/// in progress).
	pub fn get_context(&self, _entity: Entity) -> Result<&RouteContext> {
		self.context
			.as_deref()
			.ok_or_else(|| bevyhow!("no RouteContext: not inside a render"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn get_context_reads_resource() {
		let mut world = World::new();
		world.insert_resource(RouteContext::new(
			"docs/intro",
			ArticleMeta::default(),
		));
		let entity = world.spawn_empty().id();
		world
			.run_system_cached_with(
				|entity: In<Entity>, query: RenderQuery| {
					query
						.get_context(*entity)
						.unwrap()
						.current_path()
						.to_string()
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
