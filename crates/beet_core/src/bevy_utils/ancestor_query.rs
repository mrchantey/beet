use crate::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;



#[derive(SystemParam)]
pub struct AncestorQuery<
	'w,
	's,
	D: 'static + QueryData,
	F: 'static + QueryFilter = (),
> {
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
}
