//! Runtime support for `#[scene(system)]`: a build-time [`Scene`] that reads
//! the world synchronously and applies a produced sub-scene to the entity.
use beet_core::prelude::EntityWorldMutExt;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemParam;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::scene::EntityWorldMutSceneExt;
use bevy::scene::ResolveContext;
use bevy::scene::ResolveSceneError;
use bevy::scene::ResolvedScene;
use bevy::scene::Scene;
use core::marker::PhantomData;

/// The [`Scene`] emitted by a `#[scene(system)]` component.
///
/// World access cannot happen at resolve time, so this defers to the **build**
/// phase: at spawn it fetches the system params `P` against the spawning
/// entity's world, runs `build` to produce a sub-scene, and applies that
/// sub-scene to the entity. Entirely synchronous — no async constructor.
pub struct SceneSystem<P, F, S> {
	build: F,
	marker: PhantomData<fn() -> (P, S)>,
}

/// Wrap a build closure into a [`SceneSystem`]. `P` is the tuple of
/// [`SystemParam`]s the closure reads; the closure also receives the [`Entity`]
/// being built (its `In<Entity>`) and returns the sub-scene `S`.
pub fn scene_system<P, F, S>(build: F) -> SceneSystem<P, F, S>
where
	P: SystemParam + 'static,
	F: Fn(Entity, P::Item<'_, '_>) -> S + Clone + Send + Sync + 'static,
	S: Scene,
{
	SceneSystem {
		build,
		marker: PhantomData,
	}
}

impl<P, F, S> Template for SceneSystem<P, F, S>
where
	P: SystemParam + 'static,
	F: Fn(Entity, P::Item<'_, '_>) -> S + Clone + Send + Sync + 'static,
	S: Scene,
{
	type Output = ();

	fn build_template(
		&self,
		ctx: &mut TemplateContext,
	) -> bevy::ecs::error::Result<Self::Output> {
		let build = &self.build;
		// read the world synchronously and produce the sub-scene, threading the
		// entity being built so the closure can read ancestor/self context.
		let inner = ctx.entity.with_state::<P, S>(|entity, params| build(entity, params));
		// apply the produced sub-scene to this entity (immediate; no asset deps)
		ctx.entity.apply_scene(inner)?;
		Ok(())
	}

	fn clone_template(&self) -> Self {
		Self {
			build: self.build.clone(),
			marker: PhantomData,
		}
	}
}

impl<P, F, S> Scene for SceneSystem<P, F, S>
where
	P: SystemParam + Send + Sync + 'static,
	F: Fn(Entity, P::Item<'_, '_>) -> S + Clone + Send + Sync + 'static,
	S: Scene,
{
	fn resolve(
		self,
		_ctx: &mut ResolveContext,
		scene: &mut ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		scene.push_bundle_template(self);
		Ok(())
	}
}
