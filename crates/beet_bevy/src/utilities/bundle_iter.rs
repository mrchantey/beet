use crate::prelude::OnSpawn;
use beet_common_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Add to a temporary entity spawned as a child of another entity.
/// When spawned, each item in the iterator will be added as a child
/// of the parent entity, and this entity will be removed.
///
/// ## Panics
///
/// This method will remove this entity, any other bundles attempting
/// to access it after will panic.
pub fn spawn_siblings<R, B>(
	iter: impl 'static + Send + Sync + IntoIterator<Item = B>,
) -> impl Bundle
where
	R: RelationshipTarget,
	B: Bundle,
{
	OnSpawn::new(move |entity| {
		let parent = entity
			.get::<R::Relationship>()
			.expect("Spawn Siblings: Parent relationship not found")
			.get();
		let id = entity.id();
		entity.world_scope(|world| {
			for bundle in iter {
				world.spawn((
					bundle,
					<R::Relationship as Relationship>::from(parent),
				));
			}
			world.entity_mut(id).despawn();
			// defer removal to avoid panic?
			// world.commands().entity(id).despawn();
		});
	})
}

/// Like [`SpawnIter`] but for bundles, calling [`EntityWorldMut::insert`].
#[derive(ImplBundle)]
pub struct BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	pub value: T,
	_phantom: PhantomData<B>,
}

impl<T, B> BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	/// Create a new [`BundleIter`] effect.
	pub fn new(iter: T) -> Self {
		Self {
			value: iter,
			_phantom: PhantomData,
		}
	}
}

impl<T, B> BundleEffect for BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	fn apply(self, entity: &mut EntityWorldMut) {
		for bundle in self.value {
			entity.insert(bundle);
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let entity = world
			.spawn(children![spawn_siblings::<Children, Name>(vec![
				Name::new("Child1"),
				Name::new("Child2"),
			])])
			.id();
		// world.flush();

		let children = world.entity(entity).get::<Children>().unwrap();
		expect(children.len()).to_be(2);
	}
}
