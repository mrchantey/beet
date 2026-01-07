use crate::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;


/// This action will remove the specified bundle when the specified event is triggered.
/// It is designed to work for both [`GetOutcome`] and [`Outcome`] events.
/// This action also has a corresponding [`InsertOn`] action.
/// ## Example
/// Removes the `Name` bundle when the `Outcome` event is triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn((
/// 		Name::new("bill"),
/// 		EndWith(Outcome::Pass),
/// 		RemoveOn::<Outcome, Name>::default()
/// 	))
///		.trigger_target(GetOutcome);
/// ```
#[action(remove::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct RemoveOn<E: EntityTargetEvent, B: Bundle> {
	/// The target entity to remove the bundle from.
	pub target_entity: TargetEntity,
	phantom: PhantomData<(E, B)>,
}

impl<E: EntityTargetEvent, B: Bundle> Default for RemoveOn<E, B> {
	fn default() -> Self {
		Self {
			phantom: default(),
			target_entity: default(),
		}
	}
}

impl<E: EntityTargetEvent, B: Bundle> RemoveOn<E, B> {
	/// Specify the target entity for this action.
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			..default()
		}
	}
}

fn remove<E: EntityTargetEvent, B: Bundle>(
	ev: On<E>,
	mut commands: Commands,
	query: Query<&RemoveOn<E, B>>,
	agent_query: AgentQuery,
) -> Result {
	let action = ev.target();
	let remove_on = query.get(action)?;
	let target = remove_on.target_entity.get(action, &agent_query);
	commands.entity(target).remove::<B>();
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn on_run() {
		let mut app = App::new();
		app.add_plugins(ControlFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((Running, RemoveOn::<GetOutcome, Running>::default()))
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_none();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(ControlFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Running,
				RemoveOn::<Outcome, Running>::default(),
				EndWith(Outcome::Pass),
			))
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_none();
	}
}
