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
	/// Multi-purpose parameters for the handler,
	/// for instance in the case of InsertOnTrigger, this is the component to insert.
	pub params: Handler::Params,
	/// The entities to watch, defaults to [`ActionTarget::This`]
	pub source: ActionTarget,
	/// The entities to modify, defaults to [`ActionTarget::This`]
	pub target: ActionTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Handler>,
}

impl<Handler: OnTriggerHandler> Clone for OnTrigger<Handler>
where
	Handler::Params: Clone,
{
	fn clone(&self) -> Self {
		Self {
			params: self.params.clone(),
			source: self.source.clone(),
			target: self.target.clone(),
			phantom: PhantomData,
		}
	}
}

impl<Handler: OnTriggerHandler> Default for OnTrigger<Handler>
where
	Handler::Params: Default,
{
	fn default() -> Self {
		Self {
			source: Handler::default_source(),
			target: Handler::default_target(),
			params: default(),
			phantom: default(),
		}
	}
}

impl<Handler: OnTriggerHandler> MapEntities for OnTrigger<Handler> {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.source.map_entities(entity_mapper);
		self.target.map_entities(entity_mapper);
	}
}

impl<Handler: OnTriggerHandler> OnTrigger<Handler> {
	pub fn new(params: Handler::Params) -> Self {
		Self {
			params,
			..default()
		}
	}

	pub fn new_with_source(source: impl Into<ActionTarget>) -> Self {
		Self {
			source: source.into(),
			..default()
		}
	}
	pub fn new_with_target(target: impl Into<ActionTarget>) -> Self {
		Self {
			target: target.into(),
			..default()
		}
	}
	pub fn with_source(mut self, source: impl Into<ActionTarget>) -> Self {
		self.source = source.into();
		self
	}

	pub fn with_target(mut self, target: impl Into<ActionTarget>) -> Self {
		self.target = target.into();
		self
	}
}

impl<Handler: OnTriggerHandler> ActionBuilder for OnTrigger<Handler> {
	fn build(app: &mut App, _config: &BeetConfig) {
		app.world_mut()
			.register_component_hooks::<Self>()
			.on_add(|mut world, action_entity, _| {
				let action = world.get::<Self>(action_entity).unwrap();
				// use closure to capture the action entity
				let mut observer = Observer::new(
					move |trigger: Trigger<
						Handler::TriggerEvent,
						Handler::TriggerBundle,
					>,
					      query: Query<(Entity, &OnTrigger<Handler>)>,
					      mut commands: Commands| {
						let action = query
							.get(action_entity)
							.expect(expect_action::ACTION_QUERY_MISSING);
						Handler::handle(&mut commands, &trigger, action);
					},
				);
				match &action.source {
					ActionTarget::This => observer.watch_entity(action_entity),
					ActionTarget::Entity(entity) => {
						observer.watch_entity(*entity)
					}
					ActionTarget::Entities(vec) => {
						for entity in vec.iter() {
							observer.watch_entity(*entity);
						}
					}
					ActionTarget::Global => {
						// do nothing, observers are global by default.
					}
				};
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
		app.add_plugins(ActionPlugin::<InsertOnRun<Running>>::default());
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOnRun::<Running>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		Ok(())
	}
	#[test]
	fn other_sources() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<InsertOnRun<Running>>::default());
		let world = app.world_mut();

		let source = world.spawn_empty().id();
		let entity = world
			.spawn(InsertOnRun::<Running>::default().with_source(source))
			.id();

		world.entity_mut(source).flush_trigger(OnRun);

		expect(world.entities().len()).to_be(3)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		Ok(())
	}
}
