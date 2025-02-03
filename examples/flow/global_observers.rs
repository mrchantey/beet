#![allow(dead_code)]
//! An example of the general pattern used by beet in vanilla bevy
//! Hopefully this makes how beet works a bit clearer


use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Default, Component)]
struct TriggerCount(i32);

fn main() {
  let mut app = App::new();

  // these would both be automatically added
  // when the first [Health] is spawned
  app.add_observer(run_to_action::<TriggerCount>)
    .add_observer(increment);

  let start = std::time::Instant::now();
  for _ in 0..10_u64.pow(6) {
    let entity = app.world_mut().spawn(TriggerCount::default()).id();

    app.world_mut().flush();
    app.world_mut().trigger(OnRun::new(entity));
    app.world_mut().flush();
  }
  println!("Time: {}", start.elapsed().as_millis());
	// 300ms
	// assert_eq!(app.world().get::<TriggerCount>(entity).unwrap().0, 1);
}


fn increment(trigger: Trigger<OnRun>, mut query: Query<&mut TriggerCount>) {
	query.get_mut(trigger.tree).unwrap().as_mut().0 += 1;
}



/// A general observer triggered globally that can be mapped to specific actions.
#[derive(Debug, Copy, Clone, Event)]
struct OnRun {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub tree: Entity,
}

impl OnRun {
	/// Trigger [OnRun] for a target entity
	/// that will also
	pub fn new(target: Entity) -> Self {
		Self {
			target: target.clone(),
			tree: target,
		}
	}
	/// Trigger [OnRun] for a target entity
	/// that will also
	pub fn new_with_tree(target: Entity, tree: Entity) -> Self {
		Self { target, tree }
	}

	pub fn into_on_action<T: Component>(self) -> OnAction<T> {
		OnAction {
			target: self.target,
			tree: self.tree,
			_phantom: Default::default(),
		}
	}
}

/// Map a global OnRun observer to the global observer
/// watching a specific action.
/// One of these observers will be spawned for every action
/// alongside any observers it specifies.
#[derive(Event)]
struct OnAction<T> {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub tree: Entity,
	_phantom: PhantomData<T>,
}

/// Map [OnRun] to [OnAction]
fn run_to_action<T: Component>(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
) {
	commands.trigger(trigger.into_on_action::<T>());
}
