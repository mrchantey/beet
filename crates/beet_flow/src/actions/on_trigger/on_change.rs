use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use std::marker::PhantomData;


/// This is the base action for many others, ie [`EndOnRun`].
/// It supports watching multiple sources and modifying multiple targets.
/// The default behavior is to watch and modify itsself.
///
#[derive(Component, Action, Reflect)]
#[reflect(Default, Component, MapEntities)]
#[systems(on_change::<Handler>.in_set(TickSet))]
pub struct OnChange<Handler: OnChangeHandler> {
	/// The entities to watch, defaults to [`Self`] if this is empty
	pub sources: Vec<Entity>,
	/// The entities to modify, defaults to [`Self`]
	pub target: TriggerTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Handler>,
}

impl<Handler: OnChangeHandler> Clone for OnChange<Handler> {
	fn clone(&self) -> Self {
		Self {
			sources: self.sources.clone(),
			target: self.target.clone(),
			phantom: PhantomData,
		}
	}
}

impl<Handler: OnChangeHandler> Default for OnChange<Handler> {
	fn default() -> Self {
		Self {
			sources: default(),
			target: default(),
			phantom: PhantomData,
		}
	}
}

impl<Handler: OnChangeHandler> MapEntities for OnChange<Handler> {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		for entity in self.sources.iter_mut() {
			*entity = entity_mapper.map_entity(*entity);
		}
		self.target.map_entities(entity_mapper);
	}
}

impl<Handler: OnChangeHandler> OnChange<Handler> {
	pub fn new() -> Self { Self::default() }
	pub fn new_with_target(target: impl Into<TriggerTarget>) -> Self {
		Self {
			target: target.into(),
			..default()
		}
	}
	pub fn new_with_source(source: Entity) -> Self {
		Self {
			sources: vec![source],
			..default()
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

fn on_change<Handler: OnChangeHandler>(
	query: Query<
		(Entity, &Handler::ChangedComponent),
		Changed<Handler::ChangedComponent>,
	>,
	mut commands: Commands,
) {
	for entity in query.iter() {
		Handler::handle(&mut commands, entity.0, &entity.1);
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
