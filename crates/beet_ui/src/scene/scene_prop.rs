//! Type-erased scene content passed to a component as a prop.
//!
//! The runtime backing for `<slot>` composition: caller content (`<Card>…</Card>`)
//! is lowered by `rsx!` into a [`SceneProp`] and handed to the widget as its
//! `children` prop (or a named prop for additional insertion points). The widget
//! exposes the insertion points as `<slot>`s, which the `#[scene]` macro hoists
//! into `SceneProp` props and `rsx!` lowers to read them — so composition is
//! direct and non-destructive rather than a post-spawn graph rewrite.
use beet_core::prelude::*;
use bevy::scene::ResolveContext;
use bevy::scene::ResolveSceneError;
use bevy::scene::ResolvedScene;
use bevy::scene::Scene;
use bevy::scene::SceneDependencies;
use std::sync::Arc;
use std::sync::Mutex;

/// Type-erased caller content for a component prop. The default value is an
/// empty scene, so an unset prop renders nothing.
///
/// Holds the scene once (take-on-resolve): resolving consumes the inner scene so
/// a prop renders a single time. This is what lets [`SceneProp`] be [`Clone`]
/// (required by `#[scene(system)]`, whose props are cloned into the build
/// closure) while wrapping a non-`Clone` [`Box<dyn Scene>`].
#[derive(Clone, Default)]
pub struct SceneProp(Arc<Mutex<Option<Box<dyn Scene>>>>);

impl SceneProp {
	/// Wrap caller content as a scene prop.
	pub fn new(scene: impl Scene) -> Self {
		Self(Arc::new(Mutex::new(Some(Box::new(scene)))))
	}

	/// Whether any content was provided (vs the default empty prop). Reads the
	/// current state, so it must be called before the prop is placed/resolved.
	pub fn is_empty(&self) -> bool { self.0.lock().unwrap().is_none() }

	/// Fall back to `fallback` when no content was provided, mirroring a
	/// `<slot>`'s default children.
	pub fn or(self, fallback: impl Scene) -> Self {
		if self.is_empty() { Self::new(fallback) } else { self }
	}
}

impl Scene for SceneProp {
	fn resolve(
		self,
		context: &mut ResolveContext,
		scene: &mut ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		if let Some(inner) = self.0.lock().unwrap().take() {
			inner.resolve(context, scene)?;
		}
		Ok(())
	}

	fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
		if let Some(inner) = self.0.lock().unwrap().as_ref() {
			inner.register_dependencies(dependencies);
		}
	}
}
