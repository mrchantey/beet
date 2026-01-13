use crate::prelude::*;
use beet_core::prelude::*;



/// This action will insert the provided bundle when the specified event is triggered.
/// It is designed to work for both [`GetOutcome`] and [`Outcome`] events.
/// This action also has a corresponding [`RemoveOn`] action.
/// ## Example
/// Inserts the `Running` bundle when the `GetOutcome` event is triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn(InsertOn::<GetOutcome, Running>::default())
///		.trigger_target(GetOutcome);
/// ```
#[action(insert::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct InsertOn<E: EntityTargetEvent, B: Bundle + Clone> {
	/// The bundle to be cloned and inserted.
	pub bundle: B,
	/// The target entity to insert the bundle into.
	pub target_entity: TargetEntity,
	phantom: PhantomData<E>,
}

impl<E: EntityTargetEvent> InsertOn<E, OnSpawnClone> {
	pub fn new_func<B: Bundle>(
		bundle: impl 'static + Send + Sync + Clone + FnOnce() -> B,
	) -> Self {
		Self {
			bundle: OnSpawnClone::new(move |entity| {
				entity.insert(bundle.clone()());
			}),
			phantom: PhantomData,
			target_entity: TargetEntity::default(),
		}
	}
}
impl<E: EntityTargetEvent, B: Bundle + Clone> InsertOn<E, B> {
	/// Specify the bundle to be inserted
	pub fn new(bundle: B) -> Self {
		Self {
			bundle,
			phantom: PhantomData,
			target_entity: TargetEntity::default(),
		}
	}
	/// Specify the bundle to be inserted and the target entity.
	pub fn new_with_target(bundle: B, target_entity: TargetEntity) -> Self {
		Self {
			bundle,
			phantom: PhantomData,
			target_entity,
		}
	}
}

impl<E: EntityTargetEvent, B: Bundle + Clone + Default> Default
	for InsertOn<E, B>
{
	fn default() -> Self {
		Self {
			bundle: default(),
			phantom: default(),
			target_entity: default(),
		}
	}
}

fn insert<E: EntityTargetEvent, B: Bundle + Clone>(
	ev: On<E>,
	mut commands: Commands,
	query: Query<&InsertOn<E, B>>,
	agent_query: AgentQuery,
) -> Result {
	let action = ev.target();
	let insert_on = query.get(action)?;
	let target = insert_on.target_entity.get(action, &agent_query);
	commands.entity(target).insert(insert_on.bundle.clone());
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn on_run() {
		let mut app = App::new();
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOn::<GetOutcome, Running>::default())
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_some();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(ControlFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				InsertOn::<Outcome, Running>::default(),
				EndWith(Outcome::Pass),
			))
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_some();
	}
}
