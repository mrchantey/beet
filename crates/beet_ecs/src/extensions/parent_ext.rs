use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;


#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct NeedsParentRoot;

#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct ParentRoot(pub Entity);

impl MapEntities for ParentRoot {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}



pub fn set_parent_root(
	mut commands: Commands,
	parents: Query<&Parent>,
	query: Query<(Entity, &Parent), With<NeedsParentRoot>>,
) {
	for (entity, parent) in query.iter() {
		let root = ParentExt::get_root(parent, &parents);
		commands
			.entity(entity)
			.remove::<NeedsParentRoot>()
			.insert(ParentRoot(root));
	}
}

pub struct ParentExt;

impl ParentExt {
	pub fn get_root(parent: &Parent, parent_query: &Query<&Parent>) -> Entity {
		if let Ok(grandparent) = parent_query.get(**parent) {
			Self::get_root(grandparent, parent_query)
		} else {
			**parent
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
		app.add_systems(Update, set_parent_root);

		let world = app.world_mut();
		let grandparent = world.spawn(NeedsParentRoot).id();
		let parent = world.spawn(NeedsParentRoot).set_parent(grandparent).id();
		let child = world.spawn(NeedsParentRoot).set_parent(parent).id();


		app.update();

		expect(&app)
			.not()
			.to_have_component::<ParentRoot>(grandparent)?;
		expect(&app)
			.component(parent)?
			.to_be(&ParentRoot(grandparent))?;
		expect(&app)
			.component(child)?
			.to_be(&ParentRoot(grandparent))?;

		Ok(())
	}
}
