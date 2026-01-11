use crate::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;


/// Utilities for working with ancestors
#[derive(SystemParam)]
pub struct AncestorQuery<
	'w,
	's,
	D: 'static + QueryData,
	F: 'static + QueryFilter = (),
> {
	commands: Commands<'w, 's>,
	query: Query<'w, 's, D, F>,
	ancestors: Query<'w, 's, &'static ChildOf>,
}

impl<'w, 's, D: 'static + QueryData, F: 'static + QueryFilter>
	AncestorQuery<'w, 's, D, F>
{
	/// Get the first ancestor of the given entity that matches the query,
	/// inclusive of the given entity.
	pub fn get(
		&self,
		entity: Entity,
	) -> Result<<<D as QueryData>::ReadOnly as QueryData>::Item<'_, '_>> {
		for entity in self.ancestors.iter_ancestors_inclusive(entity) {
			if let Ok(entity_data) = self.query.get(entity) {
				return Ok(entity_data);
			}
		}
		bevybail!("No ancestor found matching query for entity {:?}", entity);
	}
	/// Get the first ancestor of the given entity that matches the query,
	/// inclusive of the given entity.
	pub fn get_mut<'a>(
		&'a mut self,
		entity: Entity,
	) -> Result<<D as QueryData>::Item<'a, 's>>
	where
		'w: 'a,
	{
		// First find the matching entity without borrowing mutably
		let matching_entity = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|&e| self.query.get(e).is_ok());

		if let Some(e) = matching_entity {
			// Now get mutable access to the found entity
			return self.query.get_mut(e).map_err(|e| e.into());
		}
		bevybail!("No ancestor found matching query for entity {:?}", entity);
	}

	/// Get the first ancestor of the given entity that matches the query,
	/// exclusive of the given entity.
	pub fn get_exclusive(
		&self,
		entity: Entity,
	) -> Result<<<D as QueryData>::ReadOnly as QueryData>::Item<'_, '_>> {
		for entity in self.ancestors.iter_ancestors(entity) {
			if let Ok(entity_data) = self.query.get(entity) {
				return Ok(entity_data);
			}
		}
		bevybail!("No ancestor found matching query for entity {:?}", entity);
	}

	/// Get the first ancestor of the given entity that matches the query,
	/// exclusive of the given entity.
	pub fn get_mut_exclusive<'a>(
		&'a mut self,
		entity: Entity,
	) -> Result<<D as QueryData>::Item<'a, 's>>
	where
		'w: 'a,
	{
		// First find the matching entity without borrowing mutably
		let matching_entity = self
			.ancestors
			.iter_ancestors(entity)
			.find(|&e| self.query.get(e).is_ok());

		if let Some(e) = matching_entity {
			// Now get mutable access to the found entity
			return self.query.get_mut(e).map_err(|e| e.into());
		}
		bevybail!("No ancestor found matching query for entity {:?}", entity);
	}

	/// insert into the root ancestor
	pub fn insert(
		&mut self,
		entity: Entity,
		component: impl Bundle,
	) -> Result<()>
	where
		'w: 's,
	{
		let root = self.ancestors.root_ancestor(entity);
		self.commands.entity(root).insert(component);
		Ok(())
	}
}
