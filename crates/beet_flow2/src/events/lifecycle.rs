pub use beet_core::prelude::*;

pub const RUN: Run<()> = Run(());
pub const SUCCESS: End<(), ()> = End::Success(());
pub const FAILURE: End<(), ()> = End::Failure(());

#[derive(AutoEntityEvent)]
#[entity_event(auto_propagate)]
pub struct Run<T: 'static + Send + Sync = ()>(pub T);
impl<T: 'static + Send + Sync + Default> Default for Run<T> {
	fn default() -> Self { Self(Default::default()) }
}


#[derive(Debug, Clone, PartialEq, Eq, AutoEntityEvent)]
#[entity_event(auto_propagate)]
pub enum End<T: 'static + Send + Sync = (), E: 'static + Send + Sync = ()> {
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




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn works() {
		let visited = Store::default();
		let done = Store::default();
		let mut world = World::new();
		world
			.spawn((
				// 1. send Run down to child
				EntityObserver::new(
					move |ev: On<Run>,
					      mut commands: Commands,
					      children: Query<&Children>| {
						if visited.get() {
							return;
						}
						visited.set(true);
						let child = children
							.get(ev.trigger().event_target())
							.unwrap()[0];
						commands.entity(child).auto_trigger(RUN);
						// println!("parent");
					},
				),
				children![
					// 2. Child receives Run, then sends End back up
					EntityObserver::new(
						|ev: On<Run>, mut commands: Commands| {
							commands
								.entity(ev.trigger().event_target())
								.auto_trigger(SUCCESS);
							// println!("child")
						},
					)
				],
				// 3. Parent received End, all done!
				EntityObserver::new(
					move |ev: On<End>, children: Query<&Children>| {
						ev.event().xpect_eq(SUCCESS);
						let child = children
							.get(ev.trigger().event_target())
							.unwrap()[0];
						ev.trigger().original_event_target().xpect_eq(child);
						done.set(true);
					},
				),
			))
			.auto_trigger(RUN);
		world.flush();
		done.get().xpect_true();
	}
}
