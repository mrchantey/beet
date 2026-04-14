use crate::prelude::*;
use beet_core::prelude::*;



/// Call the entity on spawn with the provided input,
/// discarding the outcome.
/// This component causes a tool call on the entity every
/// time it is inserted, including spawned from a scene file.
///
/// The call is made by a system, to ensure all components and
/// children have been added before calling.
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct CallOnSpawn<In, Out> {
	input: In,
	#[reflect(ignore)]
	phantom: std::marker::PhantomData<Out>,
}
impl<In, Out> CallOnSpawn<In, Out> {
	pub fn new(input: In) -> Self {
		Self {
			input,
			phantom: std::marker::PhantomData,
		}
	}
}
impl<In: Default, Out> Default for CallOnSpawn<In, Out> {
	fn default() -> Self {
		Self {
			input: default(),
			phantom: std::marker::PhantomData,
		}
	}
}

// we use system instead of hook or observer to ensure the rest of the tree
// has spawned before calling
pub fn call_on_spawn<
	In: 'static + Send + Sync + Clone,
	Out: 'static + Send + Sync,
>(
	// ev: On<Insert, CallOnSpawn<In, Out>>,
	mut commands: Commands,
	query: Query<(Entity, &CallOnSpawn<In, Out>), Added<CallOnSpawn<In, Out>>>,
) -> Result {
	for (entity, call_on_spawn) in query.iter() {
		commands
			.entity(entity)
			.call::<In, Out>(call_on_spawn.input.clone(), default());
	}
	Ok(())
}
