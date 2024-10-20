use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;



/// Inserts the provided `Bundle` on the [`TriggerOnTrigger::target`] when
/// the `EventIn` is triggered on one of the [`TriggerOnTrigger::sources`].
pub type InsertOnTrigger<EventIn, Bundle, EventInBundle = ()> =
	InsertMappedOnTrigger<DefaultMapFunc<EventIn, Bundle, EventInBundle>>;

pub type InsertMappedOnTrigger<M> = OnTrigger<InsertHandler<M>>;

#[derive(Reflect)]
pub struct InsertHandler<T: OnTriggerMapFunc>(
	#[reflect(ignore)] PhantomData<T>,
);


impl<M: OnTriggerMapFunc> OnTriggerHandler for InsertHandler<M>
where
	M::Out: Bundle + Clone,
{
	type TriggerEvent = M::Event;
	type TriggerBundle = M::TriggerBundle;
	type Params = M::Params;
	fn handle(
		commands: &mut Commands,
		trigger: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		(entity, action): (Entity, &OnTrigger<Self>),
	) {
		// log::info!("InsertOnTrigger: {:?}", std::any::type_name::<M::Out>());
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
	impl OnTriggerMapFunc for MapRunResult {
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
