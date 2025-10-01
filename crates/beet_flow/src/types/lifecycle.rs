use beet_core::prelude::*;
use std::marker::PhantomData;

/// Alias for `Run::<()>(())`
pub const RUN: Run<()> = Run(());
/// Alias for `End::<(),()>::Success(())`
pub const SUCCESS: End<(), ()> = End::Success(());
/// Alias for `End::<(),()>::Failure(())`
pub const FAILURE: End<(), ()> = End::Failure(());
/// Alias for `PreventEndPropagate::<(),()>::default()`
pub const PREVENT_END_PROPAGATE: PreventEndPropagate<(), ()> =
	PreventEndPropagate {
		_phantom: PhantomData,
	};

#[derive(EntityTargetEvent)]
pub struct Run<T: 'static + Send + Sync = ()>(pub T);
impl<T> Default for Run<T>
where
	T: 'static + Send + Sync + Default,
{
	fn default() -> Self { Self(default()) }
}


#[derive(Debug, Clone, PartialEq, Eq, EntityTargetEvent)]
#[entity_event(auto_propagate)]
pub enum End<T = (), E = ()>
where
	T: 'static + Send + Sync,
	E: 'static + Send + Sync,
{
	Success(T),
	Failure(E),
}

impl<T: Default, E> Default for End<T, E>
where
	T: 'static + Send + Sync,
	E: 'static + Send + Sync,
{
	fn default() -> Self { Self::Success(Default::default()) }
}

#[derive(Component)]
#[component(on_add=prevent_auto_propagate::<End<T,E>>)]
pub struct PreventEndPropagate<
	T: 'static + Send + Sync = (),
	E: 'static + Send + Sync = (),
> {
	_phantom: PhantomData<(T, E)>,
}
impl<T, E> Default for PreventEndPropagate<T, E>
where
	T: 'static + Send + Sync,
	E: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[action(run_child, exit_on_result)]
	#[derive(Component)]
	struct Parent;

	fn run_child(
		ev: On<Run>,
		mut commands: Commands,
		children: Query<&Children>,
	) {
		let child = children.get(ev.trigger().event_target()).unwrap()[0];
		commands.entity(child).trigger_target(RUN);
	}

	fn exit_on_result(
		ev: On<End>,
		mut commands: Commands,
		children: Query<&Children>,
	) {
		ev.event().xpect_eq(SUCCESS);
		let child = children.get(ev.trigger().event_target()).unwrap()[0];
		ev.trigger().original_event_target().xpect_eq(child);
		commands.write_message(AppExit::Success);
	}

	#[action(succeed)]
	#[derive(Component)]
	// #[require(PreventEndPropagate)]
	struct Child;

	fn succeed(ev: On<Run>, mut commands: Commands) {
		commands
			.entity(ev.trigger().event_target())
			.trigger_target(SUCCESS);
	}

	#[test]
	fn works() {
		let mut world = World::new();
		world.insert_resource(Messages::<AppExit>::default());
		world.spawn((Parent, children![Child])).trigger_target(RUN);
		world.should_exit().xpect_eq(Some(AppExit::Success));
	}
	#[test]
	fn prevent_propagate() {
		let mut world = World::new();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, children![(Child, PREVENT_END_PROPAGATE)]))
			.trigger_target(RUN);
		world.should_exit().xpect_none();
	}
}
