//! Components required for long running actions.
use crate::prelude::*;
use beet_core::prelude::*;


/// This will add the [`Running`] component to the behavior when [`GetOutcome`] is triggered,
/// and remove it when [`Outcome`] is triggered.
///
/// This should be added as `#[require(ContinueRun)]` for any long running action,
/// ie any action that has a [`With<Running>`] query filter.
/// It should not added to behaviors directly, because its easy to forget.
/// For usage see the [`Running`] component.
#[action(start_running, stop_running)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer)]
pub struct ContinueRun;


fn start_running(
	ev: On<GetOutcome>,
	mut commands: Commands,
	mut running: Query<&mut Running>,
) {
	if let Ok(mut running) = running.get_mut(ev.action()) {
		running.0.push(ev.agent());
	} else {
		commands
			.entity(ev.action())
			.insert(Running(vec![ev.agent()]));
	}
}

fn stop_running(
	ev: On<Outcome>,
	mut running: Query<&mut Running>,
	mut commands: Commands,
) {
	if let Ok(mut running) = running.get_mut(ev.action()) {
		running.retain(&mut commands, ev.action(), ev.agent());
	}
}


/// A marker component added to an [ActionEntity] indicate this action is currently running.
/// ## Example
/// This is the `Translate` action found in `beet_spatial`.
/// ```
///	# use beet_core::prelude::*;
///	# use beet_flow::prelude::*;
///
/// #[derive(Component)]
/// #[require(ContinueRun)]
/// struct Translate(pub Vec3);
///
/// fn translate(
/// 	time: Res<Time>,
/// 	action: Query<(Entity, &Running, &Translate)>,
/// 	mut transforms: Query<&mut Transform>,
/// ){
/// 	for (entity, _running, translate) in action.iter(){
/// 		if let Ok(mut transform) = transforms.get_mut(entity) {
/// 			transform.translation += translate.0 * time.delta_secs();
/// 		}
/// 	}
/// }
/// ```
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Debug, Default, Clone, Deref, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[require(RunTimer)]
pub struct Running(
	// custom relations until self-referential and many-many
	pub(crate) Vec<Entity>,
);


impl Running {
	/// removes the given agent, and removes the Running component
	/// if none left
	pub fn retain(
		&mut self,
		commands: &mut Commands,
		action: Entity,
		agent: Entity,
	) {
		self.0.retain(|e| *e != agent);
		if self.0.is_empty() {
			commands.entity(action).remove::<Running>();
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn adds() {
		let mut app = App::new();
		// app.add_plugins(ControlFlowPlugin::default());
		let world = app.world_mut();

		// adds
		let entity = world.spawn(ContinueRun).id();
		world.get::<Running>(entity).xpect_none();
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world.get::<Running>(entity).xpect_some();
		world
			.entity_mut(entity)
			.trigger_target(Outcome::Pass)
			.flush();
		world.get::<Running>(entity).xpect_none();
	}
}
