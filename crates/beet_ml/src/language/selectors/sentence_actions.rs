use super::Sentence;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::component::ComponentHooks;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Reflect)]
pub struct MapUserMessageToSentence;
impl MapFunc for MapUserMessageToSentence {
	type Event = OnUserMessage;
	type Params = ();
	type Out = Sentence;
	fn map(ev: &Trigger<Self::Event, Self::TriggerBundle>, _params: &Self::Params) -> Self::Out {
		log::info!("setting user message: {:?}", ev.event());
		Sentence::new(ev.event().0.clone())
	}
}


pub type SetSentenceOnUserInput =
	InsertMappedOnTrigger<MapUserMessageToSentence>;

pub type RunOnSentenceChange = TriggerOnTrigger<OnInsert, OnRun, Sentence>;


#[derive(Reflect)]
struct SerializeableObserver<T: Event>(PhantomData<T>);

impl<T: Event> Component for SerializeableObserver<T> {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	fn register_component_hooks(hooks: &mut ComponentHooks) {
		hooks.on_add(|mut world, entity, _| {
			world
				.commands()
				.entity(entity)
				.observe(|_trigger: Trigger<T>| {
					println!("it triggered etc.");
				});
		});
	}
}
