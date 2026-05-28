//! Lift plain values into `impl Scene` for use in `rsx!` child positions.
//!
//! Mirrors `crate::types::IntoBundle` — same marker-type pattern, just
//! targeting Bevy's `Scene` instead of `Bundle`. The scene `rsx!` lowering
//! routes text + block child positions through `.into_scene()` so authors can
//! write `<p>"Title: " {title} </p>` instead of `template_value(Value::new(_))`.
use crate::prelude::Classes;
use beet_core::prelude::*;
use bevy::prelude::ChildOf;
use bevy::scene::RelatedScenes;
use bevy::scene::Scene;
use bevy::scene::template_value;
use variadics_please::all_tuples;

/// Lift `self` into an [`impl Scene`]. The marker `M` disambiguates the
/// blanket impls (mirrors [`crate::types::IntoBundle`]).
pub trait IntoScene<M> {
	fn into_scene(self) -> impl Scene;
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

/// Marker for the [`Classes`] block-attribute lift.
pub struct SceneClassesMarker;

/// `{Classes::new(["btn"])}` as a block attribute lifts into a
/// [`template_value`] of the [`Classes`] component.
impl IntoScene<(NotSceneMarker, SceneClassesMarker)> for Classes {
	fn into_scene(self) -> impl Scene { template_value(self) }
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
		// classes block-attribute lift
		is_scene(Classes::new(["btn", "btn-error"]));
	}
}
