//! Lift plain values into `impl Scene` for use in `rsx!` child positions.
//!
//! Mirrors `crate::types::IntoBundle` — same marker-type pattern, just
//! targeting Bevy's `Scene` instead of `Bundle`. The scene `rsx!` lowering
//! routes text + block child positions through `.into_scene()` so authors can
//! write `<p>"Title: " {title} </p>` instead of `template_value(Value::new(_))`.
use crate::prelude::Attribute;
use crate::prelude::AttributeOf;
use beet_core::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::prelude::ChildOf;
use bevy::scene::EntityScene;
use bevy::scene::RelatedScenes;
use bevy::scene::ResolveContext;
use bevy::scene::ResolveSceneError;
use bevy::scene::ResolvedScene;
use bevy::scene::Scene;
use bevy::scene::template_value;
use std::sync::Arc;
use std::sync::Mutex;
use variadics_please::all_tuples;

/// Lift `self` into an [`impl Scene`]. The marker `M` disambiguates the
/// blanket impls (mirrors [`crate::types::IntoBundle`]).
pub trait IntoScene<M> {
	fn into_scene(self) -> impl Scene;
}

/// Build a single HTML attribute as a [`Scene`], for use as an rsx! block
/// attribute: `<a {attr("href", url)}/>`. Attributes accumulate, so this sits
/// alongside the element's literal attributes rather than replacing them.
///
/// Pair it with [`Option`] for an attribute that disappears when absent —
/// see [`optional_attr`], which is the ergonomic form for optional props.
pub fn attr(key: impl Into<String>, value: impl Into<Value>) -> impl Scene {
	RelatedScenes::<AttributeOf, _>::new(EntityScene((
		template_value(Attribute::new(key)),
		template_value(Value::new(value)),
	)))
}

/// An HTML attribute that renders only when its value is [`Some`], for use as
/// an rsx! block attribute: `<input {optional_attr("name", name)}/>` where
/// `name: Option<String>`.
///
/// A [`None`] renders nothing — unlike a defaulted empty string, which would
/// emit an incorrect `name=""`. This is the ergonomic answer to "this prop is
/// optional, so its attribute should be absent when unset".
pub fn optional_attr(
	key: impl Into<String>,
	value: Option<impl Into<Value>>,
) -> impl Scene {
	value.map(|value| attr(key, value)).into_scene()
}

/// Erase any [`Scene`] into a [`Box<dyn Scene>`] via method chain.
///
/// Useful where match arms produce differently-shaped trees and `impl Trait`
/// cannot unify — `rsx!{ <a/> }.any_scene()` reads more naturally than
/// `Box::new(rsx!{ <a/> }) as Box<dyn Scene>`. [`Box<dyn Scene>`] itself
/// implements [`Scene`] via `bevy_scene`'s `SceneBox` machinery.
#[extend::ext(name = SceneExt)]
pub impl<S: Scene> S {
	fn any_scene(self) -> Box<dyn Scene> { Box::new(self) }
}


/// Marker for the pass-through impl on existing [`Scene`] values.
pub struct SceneMarker;
/// All non-scene impls begin with this to distinguish from scene markers in
/// variadic tuples (mirrors [`crate::types::NotBundleMarker`]).
pub struct NotSceneMarker;
/// Marker for primitives that flow through `Into<Value>`.
pub struct SceneIntoValueMarker;
/// Marker that distinguishes variadic tuple impls from blanket impls.
pub struct SceneTupleMarker;

impl<S: Scene> IntoScene<SceneMarker> for S {
	fn into_scene(self) -> impl Scene { self }
}

/// Primitives (`&str`, `String`, numerics, `bool`, …) become a [`Value`]
/// template patch.
impl<T: Into<Value>> IntoScene<(NotSceneMarker, SceneIntoValueMarker)> for T {
	fn into_scene(self) -> impl Scene { template_value(Value::new(self)) }
}

/// Marker for the [`Component`] block-attribute lift.
pub struct SceneComponentMarker;

/// Any [`Component`] (eg [`Classes`](crate::prelude::Classes),
/// [`FieldRef`](crate::document::FieldRef))
/// used as a block attribute lifts into a [`template_value`] patch that
/// attaches it to the current entity. `Option<C>` composes for free via the
/// `Option` impl, so widgets can take an optional `field: Option<FieldRef>`.
impl<C> IntoScene<(NotSceneMarker, SceneComponentMarker)> for C
where
	C: 'static + Send + Sync + Unpin + Default + Clone + Component,
{
	fn into_scene(self) -> impl Scene { template_value(self) }
}

/// A [`BundleEffect`] like [`OnSpawn`] lifts into a scene that applies the
/// effect to the spawning entity at build time. This lets effect-returning
/// helpers (eg [`inline_class`](crate::prelude::inline_class)) be used as scene
/// block attributes without wrapping them in a component.
impl IntoScene<(NotSceneMarker, Self)> for OnSpawn {
	fn into_scene(self) -> impl Scene {
		OnSpawnScene(Arc::new(Mutex::new(Some(self))))
	}
}

/// Applies a take-once [`OnSpawn`] effect to the spawning entity during the
/// build phase, mirroring bevy's `OnTemplate`. The [`Arc`]/[`Mutex`] make it
/// [`Clone`] (required by [`Template`]) while holding the non-`Clone` effect.
struct OnSpawnScene(Arc<Mutex<Option<OnSpawn>>>);

impl Template for OnSpawnScene {
	type Output = ();
	fn build_template(
		&self,
		ctx: &mut TemplateContext,
	) -> bevy::ecs::error::Result<()> {
		if let Some(on_spawn) = self.0.lock().unwrap().take() {
			(on_spawn.0)(ctx.entity);
		}
		Ok(())
	}
	fn clone_template(&self) -> Self { Self(self.0.clone()) }
}

impl Scene for OnSpawnScene {
	fn resolve(
		self,
		_ctx: &mut ResolveContext,
		scene: &mut ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		scene.push_bundle_template(self);
		Ok(())
	}
}

/// `Option<T>` — present items become a scene, absent ones become an empty
/// no-op scene. Both arms are type-erased via [`Box<dyn Scene>`] because the
/// match arms produce different types.
impl<T, M> IntoScene<(NotSceneMarker, (Option<T>, M))> for Option<T>
where
	T: 'static + Send + Sync + IntoScene<M>,
	M: 'static,
{
	fn into_scene(self) -> impl Scene {
		match self {
			Some(item) => Box::new(item.into_scene()) as Box<dyn Scene>,
			None => Box::new(()) as Box<dyn Scene>,
		}
	}
}

/// `Vec<T>` — children list. Each item resolves into its own entity related
/// back to the current scene via [`ChildOf`]. `Vec<S: Scene>` already
/// implements [`SceneList`](bevy::scene::SceneList).
impl<T, M> IntoScene<(NotSceneMarker, (Vec<T>, M))> for Vec<T>
where
	T: 'static + Send + Sync + IntoScene<M>,
	M: 'static,
{
	fn into_scene(self) -> impl Scene {
		RelatedScenes::<ChildOf, _>::new(
			self.into_iter()
				.map(|item| item.into_scene())
				.collect::<Vec<_>>(),
		)
	}
}

macro_rules! impl_into_scene_tuple {
	($(($T:ident, $t:ident, $M:ident)),*) => {
		impl<$($T, $M),*> IntoScene<(SceneTupleMarker,($($M,)*))> for ($($T,)*)
		where
			$($T: IntoScene<(NotSceneMarker, $M)>,)*
		{
			fn into_scene(self) -> impl Scene {
				let ($($t,)*) = self;
				($($t.into_scene(),)*)
			}
		}
	}
}

all_tuples!(impl_into_scene_tuple, 2, 12, T, t, M);

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	fn is_scene<M>(_: impl IntoScene<M>) {}

	#[beet_core::test]
	fn works() {
		// pass-through: anything already a Scene
		is_scene(template_value(Value::new("hi")));
		// primitives via Into<Value>
		is_scene("hello");
		is_scene(String::from("hello"));
		is_scene(42_i32);
		is_scene(3.14_f64);
		is_scene(true);
		// vec of values (children list)
		is_scene(vec!["a", "b", "c"]);
		// tuple
		is_scene((1_i32, "two"));
		// classes block-attribute lift (a Component)
		is_scene(Classes::new(["btn", "btn-error"]));
		// arbitrary Component lift, and its optional form
		is_scene(FieldRef::new("name"));
		is_scene(Some(FieldRef::new("name")));
		is_scene(None::<FieldRef>);
	}
}
