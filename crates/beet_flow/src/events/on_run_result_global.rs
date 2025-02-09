use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

/// Signifies a behavior has stopped running. This bubbles
/// up the tree until it reaches the root node or a [`StopBubble`].
#[derive(Debug, Component, Clone, Copy, PartialEq, Reflect)]
pub struct OnRunResultGlobal {
	pub result: RunResult,
	/// The entity that triggered the behavior,
	/// control flow nodes will generally replace this
	/// with their own node entity as the event bubbles up.
	pub caller: Entity,
}

impl Event for OnRunResultGlobal {
	type Traversal = &'static Parent;
	const AUTO_PROPAGATE: bool = true;
}


impl OnRunResultGlobal {
	pub fn new(caller: Entity, result: RunResult) -> Self {
		Self { result, caller }
	}
	/// Populate with [`RunResult::Success`]
	pub fn success(caller: Entity) -> Self {
		Self {
			caller,
			result: RunResult::Success,
		}
	}
	/// Populate with [`RunResult::Failure`]
	pub fn failure(caller: Entity) -> Self {
		Self {
			caller,
			result: RunResult::Failure,
		}
	}
}

#[derive(Debug, Default, Component, Clone, Copy, PartialEq, Reflect)]
#[component(on_add=stop_bubble_on_add,on_remove=stop_bubble_on_remove)]
pub struct StopBubble(pub Option<Entity>);

fn stop_bubble_on_add(
	mut world: DeferredWorld,
	entity: Entity,
	_cid: ComponentId,
) {
	let observer = world
		.commands()
		.spawn(
			Observer::new(|mut trigger: Trigger<OnRunResultGlobal>| {
				trigger.propagate(false);
			})
			.with_entity(entity),
		)
		.id();
	world
		.commands()
		.entity(entity)
		.insert(StopBubble(Some(observer)));
}

/// we'll try to clean up the observer but if it isnt there just
/// fail silently
fn stop_bubble_on_remove(
	mut world: DeferredWorld,
	entity: Entity,
	_cid: ComponentId,
) {
	if let Some(observer) =
		world.get::<StopBubble>(entity).map(|e| e.0).flatten()
	{
		world.commands().entity(observer).try_despawn();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn bubbles_up() {
		let mut world = World::default();
		let counter = observe_triggers::<OnRunResultGlobal>(&mut world);

		world.spawn_empty().with_child(()).with_children(|parent| {
			parent.spawn_empty().with_children(|parent| {
				parent
					.spawn_empty()
					.trigger(OnRunResultGlobal::success(Entity::PLACEHOLDER));
			});
		});
		world.flush();

		expect(&counter).to_have_been_called_times(3);
	}
	#[test]
	fn stop_bubble() {
		let mut world = World::default();
		let counter = observe_triggers::<OnRunResultGlobal>(&mut world);

		let mut grandchild = Entity::PLACEHOLDER;
		let mut child = Entity::PLACEHOLDER;

		world.spawn_empty().with_child(()).with_children(|parent| {
			child = parent
				.spawn(StopBubble::default())
				.with_children(|parent| {
					grandchild = parent.spawn_empty().id();
				})
				.id();
		});

		world
			.entity_mut(grandchild)
			.flush_trigger(OnRunResultGlobal::success(child));

		// only child and grandchild called
		expect(&counter).to_have_been_called_times(2);

		world.entity_mut(child).remove::<StopBubble>();
		world
			.entity_mut(grandchild)
			.flush_trigger(OnRunResultGlobal::success(child));
		// it was removed so all called
		expect(&counter).to_have_been_called_times(5);
	}
}
