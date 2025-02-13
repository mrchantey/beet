use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


/// This action will remove the specified bundle when the specified action is triggered.
/// It is designed to work for both [`OnRun`] and [`OnResult`] events.
/// This action also has a corresponding [`Insert`] action.
/// ## Example
/// Removes the `Running` bundle when the `OnResult` event is triggered.
/// ```
/// # use beet_flow::prelude::*;
/// # let mut world = world();
/// world
///		.spawn((
/// 		ReturnWith(RunResult::Success),
/// 		Remove::<OnResult, Running>::default()
/// 	))
///		.trigger(OnRun::local());
/// ```
#[action(remove::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct Remove<E: ObserverEvent, B: Bundle> {
	phantom: PhantomData<(E, B)>,
}

impl<E: ObserverEvent, B: Bundle + Default> Default for Remove<E, B> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

fn remove<E: ObserverEvent, B: Bundle>(ev: Trigger<E>, mut commands: Commands) {
	commands.entity(ev.action()).remove::<B>();
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn on_run() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((Running::default(), Remove::<OnRun, Running>::default()))
			.flush_trigger(OnRun::local())
			.id();
		expect(world.get::<Running>(entity)).to_be_none();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Running::default(),
				Remove::<OnResult, Running>::default(),
				ReturnWith(RunResult::Success),
			))
			.flush_trigger(OnRun::local())
			.id();
		expect(world.get::<Running>(entity)).to_be_none();
	}
}
