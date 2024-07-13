use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub type InsertOnTrigger<Event, Params, TriggerBundle = ()> =
	InsertMappedOnTrigger<DefaultMapFunc<Event, Params, TriggerBundle>>;

pub type InsertMappedOnTrigger<M> = OnTrigger<InsertHandler<M>>;

#[derive(Reflect)]
pub struct InsertHandler<T: MapFunc>(#[reflect(ignore)] PhantomData<T>);


impl<M: MapFunc> OnTriggerHandler for InsertHandler<M>
where
	M::Out: Bundle + Clone,
{
	type Event = M::Event;
	type TriggerBundle = M::TriggerBundle;
	type Params = M::Params;
	fn handle(
		commands: &mut Commands,
		trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		(entity, action): (Entity, &OnTrigger<Self>),
	) {
		let out = M::map(trigger, (entity, &action.params));
		action.target.insert(commands, entity, out);
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
		app.add_plugins(
			ActionPlugin::<InsertOnTrigger<OnRun, Running>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOnTrigger::<OnRun, Running>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		Ok(())
	}

	#[derive(Reflect)]
	struct MapRunResult;
	impl MapFunc for MapRunResult {
		type Event = OnRun;
		type Params = RunResult;
		type Out = Name;
		fn map(
			_ev: &Trigger<Self::Event>,
			(_, params): (Entity, &Self::Params),
		) -> Self::Out {
			Name::new(format!("{:?}", params))
		}
	}

	#[test]
	fn with_map() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<InsertMappedOnTrigger<MapRunResult>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn(InsertMappedOnTrigger::<MapRunResult>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&*world)
			.component(entity)?
			.to_be(&Name::new("Success"))?;
		Ok(())
	}
}
