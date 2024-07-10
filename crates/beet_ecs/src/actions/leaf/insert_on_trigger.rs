use crate::prelude::*;
use bevy::prelude::*;
use leaf::component_target::ComponentTarget;
use std::marker::PhantomData;


pub trait MapFunc: 'static + Send + Sync {
	type Event: Event;
	type Params: 'static + Send + Sync + Clone + Default + Reflect;
	type Out: Bundle;
	fn map(ev: Trigger<Self::Event>, params: Self::Params) -> Self::Out;
}
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct DefaultMapFunc<E, T>(#[reflect(ignore)] PhantomData<(E, T)>);
impl<E: Event, T: Bundle + Clone + Default + Reflect> MapFunc
	for DefaultMapFunc<E, T>
{
	type Event = E;
	type Params = T;
	type Out = T;
	fn map(_ev: Trigger<E>, params: T) -> T { params }
}


pub type InsertOnTrigger<Event, Params> =
	InsertMappedOnTrigger<DefaultMapFunc<Event, Params>>;


/// Adds the provided component when [`<E>`] is triggered
#[derive(Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_trigger::<Map>)]
pub struct InsertMappedOnTrigger<Map: MapFunc> {
	pub params: Map::Params,
	pub target: ComponentTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Map>,
}

impl<Map: MapFunc> Default for InsertMappedOnTrigger<Map>
where
	Map::Params: Default,
{
	fn default() -> Self { Self::new(Map::Params::default()) }
}

impl<Map: MapFunc> InsertMappedOnTrigger<Map> {
	pub fn new(params: Map::Params) -> Self {
		Self {
			params,
			target: default(),
			phantom: PhantomData,
		}
	}
	pub fn with_target(self, target: impl Into<ComponentTarget>) -> Self {
		Self {
			target: target.into(),
			..self
		}
	}
}

fn on_trigger<Map: MapFunc>(
	trigger: Trigger<Map::Event>,
	query: Query<&InsertMappedOnTrigger<Map>>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	let target = trigger.entity();
	let out = Map::map(trigger, action.params.clone());
	action.target.insert(&mut commands, target, out);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		let entity = world
			.spawn(InsertOnTrigger::<OnRun, Running>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&world).to_have_component::<Running>(entity)?;
		Ok(())
	}

	struct MapRunResult;
	impl MapFunc for MapRunResult {
		type Event = OnRun;
		type Params = RunResult;
		type Out = Name;
		fn map(_ev: Trigger<Self::Event>, params: Self::Params) -> Self::Out {
			Name::new(format!("{:?}", params))
		}
	}

	#[test]
	fn with_map() -> Result<()> {
		let mut world = World::new();

		let entity = world
			.spawn(InsertMappedOnTrigger::<MapRunResult>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&world)
			.component(entity)?
			.to_be(&Name::new("Success"))?;
		Ok(())
	}
}
