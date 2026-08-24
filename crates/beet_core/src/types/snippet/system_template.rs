//! Runtime support for `#[template(system)]`: a build-subtree [`Template`] that
//! reads the world synchronously at build time and applies a produced subtree to
//! the entity.
use crate::prelude::*;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemParam;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use core::marker::PhantomData;

/// The [`Template`] a `#[template(system)]` component builds into.
///
/// World access happens at the build phase: it fetches the system params `P`
/// against the building entity (the established
/// [`with_state`](crate::prelude::EntityWorldMutExt::with_state) pattern),
/// runs `build` to produce a child template, and builds that subtree into the
/// entity. Entirely synchronous, no async constructor.
pub struct SystemTemplate<P, F, T> {
	build: F,
	marker: PhantomData<fn() -> (P, T)>,
}

impl<P, F, T> SystemTemplate<P, F, T>
where
	P: SystemParam + 'static,
	F: Fn(Entity, P::Item<'_, '_>) -> Result<T> + Clone + Send + Sync + 'static,
	T: Template<Output = ()>,
{
	/// Wrap a build closure into a [`SystemTemplate`]. `P` is the tuple of
	/// [`SystemParam`]s the closure reads; the closure also receives the
	/// [`Entity`] being built so it can read self/ancestor context, and returns
	/// the child template `T`. The closure is fallible, so a build that reads
	/// the world *and* can fail (parsing a declared uri against a resource) is
	/// raised through the build's own error channel rather than unwrapped.
	pub fn new(build: F) -> Self {
		SystemTemplate {
			build,
			marker: PhantomData,
		}
	}
}

impl<P, F, T> Template for SystemTemplate<P, F, T>
where
	P: SystemParam + 'static,
	F: Fn(Entity, P::Item<'_, '_>) -> Result<T> + Clone + Send + Sync + 'static,
	T: Template<Output = ()>,
{
	type Output = ();

	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		let build = &self.build;
		// read the world synchronously and produce the child template, threading
		// the building entity so the closure can read self/ancestor context.
		let inner =
			cx.entity.with_state::<P, Result<T>>(|entity, params| {
				build(entity, params)
			})?;
		// build the produced subtree into this entity (synchronous, no lifecycle).
		cx.entity.build_template(&inner)
	}

	fn clone_template(&self) -> Self {
		Self {
			build: self.build.clone(),
			marker: PhantomData,
		}
	}
}
