use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::ecs::system::IntoObserverSystem;

pub trait IntoBundle<M> {
	fn into_bundle(self) -> impl Bundle;
}

pub struct BundleMarker;

impl<T: Bundle> IntoBundle<BundleMarker> for T {
	fn into_bundle(self) -> impl Bundle { self }
}

#[extend::ext(name = AnyBundleExt)]
pub impl<T, M> T
where
	T: IntoBundle<M>,
{
	fn any_bundle(self) -> OnSpawn {
		let bundle = self.into_bundle();
		OnSpawn::new(move |entity| {
			entity.insert(bundle);
		})
	}
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

impl<T, E, B: Bundle, M> IntoBundle<(ObserverMarker, E, B, M)> for T
where
	E: Event,
	B: Bundle,
	T: 'static + Send + Sync + IntoObserverSystem<E, B, M>,
{
	fn into_bundle(self) -> impl Bundle { OnSpawn::observe(self) }
}

impl<T, M> IntoBundle<(Self, M)> for Option<T>
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

impl<T, M> IntoBundle<(Self, M)> for Vec<T>
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
impl IntoBundle<Self> for Entity {
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

pub struct ValueMarker;

impl<T: Into<Value>> IntoBundle<ValueMarker> for T {
	fn into_bundle(self) -> impl Bundle { self.into() }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		fn is_bundle<M>(_: impl IntoBundle<M>) {}
		#[derive(Event)]
		struct Foo;
		is_bundle(());
		is_bundle(Name::new("foo"));
		is_bundle(|_: On<Foo>| {});
		is_bundle(Entity::PLACEHOLDER);
		is_bundle(0_i32);
		is_bundle("foo");
	}
}
