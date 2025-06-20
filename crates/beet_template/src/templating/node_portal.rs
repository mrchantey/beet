use bevy::platform::collections::HashMap;
use bevy::prelude::*;


/// Flag component to accompany a [`NodePortal`] used for query filtering.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PortalTo<T> {
	phantom: std::marker::PhantomData<T>,
}
impl<T> Default for PortalTo<T> {
	fn default() -> Self { Self { phantom: default() } }
}


/// A node that, like a pointer, exists only to point to another node.
/// Examples include the content of template `<style>` tags which is replaced with
/// a `NodePointer` component in order to deduplicate for the html output.
#[derive(Deref, Component, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = NodePortalTarget)]
pub struct NodePortal {
	target: Entity,
}

impl NodePortal {
	pub fn new(target: Entity) -> Self { Self { target } }
}

/// A node that is pointed to by multiple other nodes.
#[derive(Deref, Component, Reflect)]
#[reflect(Component)]
#[relationship_target(relationship = NodePortal,linked_spawn)]
pub struct NodePortalTarget {
	sources: Vec<Entity>,
}




/// Allows seperate nodes to point to the same content by using the same hash.
/// This component will be replaced with a shared `NodePortal` pointing to
/// a cloned version of the first entity encountered with the same hash.
#[derive(
	Debug, Clone, Copy, Component, PartialEq, Eq, PartialOrd, Ord, Deref,
)]
pub struct IntoPortal {
	/// a hash of everything that makes this node unique,
	/// such as the tag, attributes, directives and inner text.
	hash: u64,
}
impl IntoPortal {
	pub fn new(hash: u64) -> Self { Self { hash } }
}


pub fn into_portal_system(
	mut commands: Commands,
	query: Populated<(Entity, &IntoPortal), Added<IntoPortal>>,
) {
	let mut portal_hash = HashMap::<u64, Vec<Entity>>::new();

	for entity in query.iter() {
		portal_hash.entry(entity.1.hash).or_default().push(entity.0);
	}

	for entities in portal_hash.into_values() {
		let mut first = commands.entity(entities[0]);
		let target = first
			.clone_and_spawn_with(|_config| {
				// we will likely need to filter here
			})
			.id();
		for entity in entities.iter() {
			commands
				.entity(*entity)
				.insert(NodePortal::new(target))
				.remove::<IntoPortal>();
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_systems(Update, into_portal_system);
		let world = app.world_mut();
		let _a = world.spawn(IntoPortal::new(42)).id();
		let _b = world.spawn(IntoPortal::new(42)).id();
		app.update();
		let world = app.world_mut();
		world.flush();
		world.query_once::<&NodePortal>().len().xpect().to_be(2);
		world
			.query_once::<&NodePortalTarget>()
			.len()
			.xpect()
			.to_be(1);
	}
}
