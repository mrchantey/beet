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
#[component(on_add=prevent_auto_propagate::<true,End<T,E>,&'static ChildOf>)]
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


	fn setup(done: Store<bool>) -> impl Bundle {
		let visited = Store::default();
		(
			// 1. send Run down to child
			EntityObserver::new(
				move |ev: On<Run>,
				      mut commands: Commands,
				      children: Query<&Children>| {
					if visited.get() {
						return;
					}
					visited.set(true);
					let child =
						children.get(ev.trigger().event_target()).unwrap()[0];
					commands.entity(child).trigger_target(RUN);
					// println!("parent");
				},
			),
			children![
				// 2. Child receives Run, then sends End back up
				(
					EntityObserver::new(
						|ev: On<Run>, mut commands: Commands| {
							commands
								.entity(ev.trigger().event_target())
								.trigger_target(SUCCESS);
							// println!("child")
						}
					),
					// PREVENT_END_PROPAGATE
				)
			],
			// 3. Parent received End, all done!
			EntityObserver::new(
				move |ev: On<End>, children: Query<&Children>| {
					ev.event().xpect_eq(SUCCESS);
					let child =
						children.get(ev.trigger().event_target()).unwrap()[0];
					ev.trigger().original_event_target().xpect_eq(child);
					done.set(true);
				},
			),
		)
	}

	#[test]
	fn works() {
		let done = Store::default();
		let mut world = World::new();
		world.spawn(setup(done)).trigger_target(RUN);
		done.get().xpect_true();
	}
}
