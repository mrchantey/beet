#![allow(dead_code)]
//! An example of the general pattern used by beet in vanilla bevy
//! Hopefully this makes how beet works a bit clearer


use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Component)]
struct Health(i32);

fn main() {
	let mut app = App::new();

	// these would both be automatically added
	// when the first [Health] is spawned
	app.add_observer(run_to_action::<Health>)
		.add_observer(log_health);

	let agent = app.world_mut().spawn(Health(1)).id();

	app.world_mut().flush();
	app.world_mut().trigger(OnRun::new(agent));
	app.world_mut().flush();
}


fn log_health(trigger: Trigger<OnAction<Health>>, query: Query<&Health>) {
	let health = query.get(trigger.target).unwrap().0;
	println!("health: {}", health);
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
