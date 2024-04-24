use crate::prelude::*;
use bevy::prelude::*;

pub trait SettableComponent:
	Default + Clone + Component + GetTypeRegistration
{
}
impl<T: Default + Clone + Component + GetTypeRegistration> SettableComponent
	for T
{
}

#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(graph_role=GraphRole::Node,set=PreTickSet)]
/// Sets a component when this behavior spawns.
/// This does nothing if the entity does not have the component.
pub struct SetOnSpawn<T: SettableComponent>(pub T);

impl<T: SettableComponent> SetOnSpawn<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_spawn<T: SettableComponent>(
	mut query: Query<(&SetOnSpawn<T>, &mut T), Added<SetOnSpawn<T>>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}

#[derive_action]
#[derive(PartialEq, Deref, DerefMut)]
#[action(graph_role=GraphRole::Node,set=PreTickSet)]
pub struct InsertOnRun<T: SettableComponent>(pub T);

impl<T: SettableComponent> InsertOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn insert_on_run<T: SettableComponent>(
	mut commands: Commands,
	query: Query<(Entity, &InsertOnRun<T>), Added<Running>>,
) {
	for (entity, from) in query.iter() {
		commands.entity(entity).insert(from.0.clone());
	}
}

/// Sets a component when this behavior starts running.
/// This does nothing if the entity does not have the component.
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(graph_role=GraphRole::Node,set=PostTickSet)]
pub struct SetOnRun<T: SettableComponent>(pub T);

impl<T: SettableComponent> SetOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_run<T: SettableComponent>(
	mut query: Query<(&SetOnRun<T>, &mut T), Added<Running>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}

/// Sets an agent's component when this behavior starts running.
/// This does nothing if the agent does not have the component.
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(graph_role=GraphRole::Agent,set=PostTickSet)]
pub struct SetAgentOnRun<T: SettableComponent>(pub T);

impl<T: SettableComponent> SetAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_agent_on_run<T: SettableComponent>(
	mut agents: Query<&mut T>,
	mut query: Query<(&TargetAgent, &SetAgentOnRun<T>), Added<Running>>,
) {
	for (entity, src) in query.iter_mut() {
		if let Ok(mut dst) = agents.get_mut(**entity) {
			*dst = src.0.clone();
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(BeetSystemsPlugin::<EcsModule, _>::default());

		let actions = test_constant_behavior_tree();
		let root = actions.build(app.world_mut()).value;

		app.world_mut()
			.entity_mut(root)
			.insert(SetOnSpawn(Score::Pass));

		expect(&app).component(root)?.to_be(&Score::Fail)?;
		app.update();
		expect(&app).component(root)?.to_be(&Score::Pass)?;

		Ok(())
	}
}
