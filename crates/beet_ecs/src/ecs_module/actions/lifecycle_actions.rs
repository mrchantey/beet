use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;


pub trait SettableComponent:
	Default + Clone + Component + GetTypeRegistration
{
}
impl<T: Default + Clone + Component + GetTypeRegistration> SettableComponent
	for T
{
}

// #[action(
// 	system=constant_score,
// 	set=PreTickSet,
// 	components=Score::default()
// )]
// #[reflect(Component, Action)]
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(graph_role=GraphRole::Node,set=PreTickSet)]
pub struct SetOnStart<T: SettableComponent>(pub T);

impl<T: SettableComponent> SetOnStart<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_start<T: SettableComponent>(
	mut query: Query<(&SetOnStart<T>, &mut T), Added<SetOnStart<T>>>,
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

// this was SetRunResult - With<Running> check for regression
fn insert_on_run<T: SettableComponent>(
	mut commands: Commands,
	query: Query<(Entity, &InsertOnRun<T>), Added<Running>>,
) {
	for (entity, from) in query.iter() {
		commands.entity(entity).insert(from.0.clone());
	}
}

/// If the node does not have the component this will do nothing.
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

/// If the agent does not have the component this will do nothing.
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(graph_role=GraphRole::Agent,set=PostTickSet)]
pub struct SetAgentOnRun<T: SettableComponent>(pub T);

impl<T: SettableComponent> SetAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_agent_on_run<T: SettableComponent>(
	mut agents: Query<&mut T>,
	mut query: Query<(&ParentRoot, &SetAgentOnRun<T>), Added<Running>>,
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
			.insert(SetOnStart(Score::Pass));

		expect(&app).component(root)?.to_be(&Score::Fail)?;
		app.update();
		expect(&app).component(root)?.to_be(&Score::Pass)?;

		Ok(())
	}
}
