use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use std::marker::PhantomData;


/// This is the base action for many others, ie [`EndOnRun`].
/// It supports watching multiple sources and modifying multiple targets.
/// The default behavior is to watch and modify itsself.
///
#[derive(Component, Reflect)]
#[reflect(Default, Component, MapEntities)]
pub struct OnTrigger<Handler: OnTriggerHandler> {
	pub params: Handler::Params,
	/// The entities to watch, defaults to [`Self`] if this is empty
	pub sources: Vec<Entity>,
	/// The entities to modify, defaults to [`Self`]
	pub target: TriggerTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Handler>,
}

impl<Handler: OnTriggerHandler> Default for OnTrigger<Handler>
where
	Handler::Params: Default,
{
	fn default() -> Self { Self::new(Handler::Params::default()) }
}

impl<Handler: OnTriggerHandler> MapEntities for OnTrigger<Handler> {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		for entity in self.sources.iter_mut() {
			*entity = entity_mapper.map_entity(*entity);
		}
		self.target.map_entities(entity_mapper);
	}
}

impl<Handler: OnTriggerHandler> OnTrigger<Handler> {
	pub fn new(params: Handler::Params) -> Self {
		Self {
			params,
			sources: default(),
			target: default(),
			phantom: PhantomData,
		}
	}
	pub fn with_target(self, target: impl Into<TriggerTarget>) -> Self {
		Self {
			target: target.into(),
			..self
		}
	}
	pub fn with_source(mut self, source: Entity) -> Self {
		self.sources.push(source);
		self
	}
}

// fn on_trigger<Handler: OnTriggerHandler>(
// 	trigger: Trigger<Handler::Event, Handler::TriggerBundle>,
// 	query: Query<(Entity, &OnTrigger<Handler>)>,
// 	mut commands: Commands,
// ) {
// 	let action = query
// 		.get(trigger.entity())
// 		.expect(expect_action::ACTION_QUERY_MISSING);
// 	Handler::handle(&mut commands, &trigger, action);
// }

impl<Handler: OnTriggerHandler> ActionBuilder for OnTrigger<Handler> {
	fn build(app: &mut App, _config: &BeetConfig) {
		app.world_mut()
			.register_component_hooks::<Self>()
			.on_add(|mut world, entity, _| {
				let action = world.get::<Self>(entity).unwrap();
				let mut observer = Observer::new(
					move |trigger: Trigger<
						Handler::TriggerEvent,
						Handler::TriggerBundle,
					>,
					      query: Query<(Entity, &OnTrigger<Handler>)>,
					      mut commands: Commands| {
						let action = query
							.get(entity)
							.expect(expect_action::ACTION_QUERY_MISSING);
						Handler::handle(&mut commands, &trigger, action);
					},
				);
				if action.sources.is_empty() {
					observer.watch_entity(entity);
				} else {
					for entity in action.sources.iter() {
						observer.watch_entity(*entity);
					}
				}
				let mut commands = world.commands();
				let entity = commands.spawn(observer).id();
				commands
					.entity(entity)
					.insert(ActionObserverMap::<Self>::new(vec![entity]));
			})
			.on_remove(|mut world, entity, _| {
				ActionObserversBuilder::cleanup::<Self>(&mut world, entity);
			});
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
	#[test]
	fn other_sources() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<InsertOnTrigger<OnRun, Running>>::default(),
		);
		let world = app.world_mut();

		let source = world.spawn_empty().id();
		let entity = world
			.spawn(
				InsertOnTrigger::<OnRun, Running>::default()
					.with_source(source),
			)
			.id();

		world.entity_mut(source).flush_trigger(OnRun);

		expect(world.entities().len()).to_be(3)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		Ok(())
	}
}
