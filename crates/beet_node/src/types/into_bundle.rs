use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::ecs::system::IntoObserverSystem;

pub trait IntoBundle<M> {
	fn into_bundle(self) -> impl Bundle;
}

pub struct BundleMarker;
// all non-bundle impls begin with this to distinguish from
// bundle markers in variadics
pub struct NotBundleMarker;

impl<T: Bundle> IntoBundle<BundleMarker> for T {
	fn into_bundle(self) -> impl Bundle { self }
}

#[extend::ext(name = AnyBundleExt)]
pub impl<T, M> T
where
	T: IntoBundle<M>,
{
	/// Type erased bundle, inserted on spawn.
	/// Useful for match statements and other conditional bundle returns
	fn any_bundle(self) -> OnSpawn { OnSpawn::insert(self.into_bundle()) }
}

#[extend::ext(name=AnyBundleCloneExt)]
pub impl<T, M> T
where
	T: 'static + Send + Sync + Clone + IntoBundle<M>,
{
	fn any_bundle_clone(self) -> OnSpawnClone {
		OnSpawnClone::insert(move || self.clone().into_bundle())
	}
}

pub struct ObserverMarker;

impl<T, E, B: Bundle, M>
	IntoBundle<(NotBundleMarker, (ObserverMarker, E, B, M))> for T
where
	E: Event,
	B: Bundle,
	T: 'static + Send + Sync + IntoObserverSystem<E, B, M>,
{
	fn into_bundle(self) -> impl Bundle { OnSpawn::observe(self) }
}

// Option
impl<T, M> IntoBundle<(NotBundleMarker, (Self, M))> for Option<T>
where
	T: IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle {
		match self {
			Some(item) => item.any_bundle(),
			None => OnSpawn::new(|_| {}),
		}
	}
}

/// Vec
impl<T, M> IntoBundle<(NotBundleMarker, (Self, M))> for Vec<T>
where
	T: 'static + Send + Sync + IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle {
		(Children::spawn(SpawnIter(
			self.into_iter().map(|item| item.into_bundle()),
		)),)
	}
}

/// Entities are convert into children of the entity they are inserted into.
///
/// `rsx!{<div>{entity}</div>}` spawns an entity with this OnSpawn effect,
/// which becomes the parent of the entity passed in.
impl IntoBundle<(NotBundleMarker, Self)> for Entity {
	fn into_bundle(self) -> impl Bundle {
		OnSpawnTyped::new(move |spawned_entity| {
			// here the spawned entity is a fragment
			let id = spawned_entity.id();
			spawned_entity.world_scope(|world| {
				world.entity_mut(self).insert(ChildOf(id));
			});
		})
	}
}

// all primitives: string, bool, u32 etc
pub struct IntoValueMarker;

impl<T: Into<Value>> IntoBundle<(NotBundleMarker, IntoValueMarker)> for T {
	fn into_bundle(self) -> impl Bundle { self.into() }
}


use variadics_please::all_tuples;

/// Marker that distinguishes variadic tuple [`IntoBundle`] impls
/// from the observer and blanket impls.
pub struct TupleMarker;

macro_rules! impl_into_bundle_tuple {
	($(($T:ident, $t:ident, $M:ident)),*) => {
		impl<$($T, $M),*> IntoBundle<(TupleMarker,($($M,)*))> for ($($T,)*)
		where
			$($T: IntoBundle<(NotBundleMarker, $M)>,)*
		{
			fn into_bundle(self) -> impl Bundle {
				let ($($t,)*) = self;
				($($t.into_bundle(),)*)
			}
		}
	}
}

all_tuples!(impl_into_bundle_tuple, 2, 15, T, t, M);

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	fn is_bundle<M>(_: impl IntoBundle<M>) {}

	#[test]
	fn works() {
		#[derive(Event)]
		struct Foo;
		#[derive(Component)]
		struct Bar;
		is_bundle(());
		is_bundle(Name::new("foo"));
		is_bundle(|_: On<Foo>| {});
		is_bundle(Entity::PLACEHOLDER);
		is_bundle(0_i32);
		is_bundle("foo");
		is_bundle((0_i32, "hello"));
		is_bundle((Entity::PLACEHOLDER, "text", 42_i32));
		is_bundle(Bar);
		is_bundle((Bar, Bar));
	}
}
