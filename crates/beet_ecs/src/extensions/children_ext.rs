use bevy::prelude::*;
use bevy::utils::HashSet;

pub struct ChildrenExt;

impl ChildrenExt {
	/// Array this entity and all of its children
	pub fn collect(
		entity: Entity,
		edge_query: &Query<&Children>,
	) -> Vec<Entity> {
		let mut entities = Vec::new();
		Self::visit(entity, edge_query, |entity| {
			entities.push(entity);
		});
		entities
	}
	pub fn collect_world(entity: Entity, world: &mut World) -> Vec<Entity> {
		let mut entities = Vec::new();
		Self::visit_dfs_world(world, entity, |_, entity| {
			entities.push(entity);
		});
		entities
	}

	/// dfs visit
	pub fn visit(
		entity: Entity,
		edge_query: &Query<&Children>,
		mut func: impl FnMut(Entity),
	) {
		Self::visit_or_cancel(entity, edge_query, |entity| {
			func(entity);
			true
		});
	}
	/// dfs visit, do not visit children if func returns false
	pub fn visit_or_cancel(
		entity: Entity,
		edge_query: &Query<&Children>,
		mut func: impl FnMut(Entity) -> bool,
	) -> bool {
		fn visit_inner(
			entity: Entity,
			edge_query: &Query<&Children>,
			func: &mut impl FnMut(Entity) -> bool,
		) -> bool {
			if !func(entity) {
				return false;
			}
			if let Ok(children) = edge_query.get(entity) {
				for child in children.iter() {
					visit_inner(*child, edge_query, func);
				}
			}
			return true;
		}
		visit_inner(entity, edge_query, &mut func)
	}

	/// dfs find
	pub fn find<T>(
		entity: Entity,
		query: &Query<&Children>,
		mut func: impl FnMut(Entity) -> Option<T>,
	) -> Option<T> {
		fn find<T>(
			entity: Entity,
			query: &Query<&Children>,
			func: &mut impl FnMut(Entity) -> Option<T>,
		) -> Option<T> {
			if let Some(val) = func(entity) {
				return Some(val);
			}
			if let Ok(children) = query.get(entity) {
				for child in children.iter() {
					match find(*child, query, func) {
						None => {}
						some => return some,
					}
				}
			}
			None
		}
		find(entity, query, &mut func)
	}
	pub fn first(
		entity: Entity,
		query: &Query<&Children>,
		mut func: impl FnMut(Entity) -> bool,
	) -> Option<Entity> {
		fn first(
			entity: Entity,
			query: &Query<&Children>,
			func: &mut impl FnMut(Entity) -> bool,
		) -> Option<Entity> {
			if func(entity) {
				return Some(entity);
			}
			if let Ok(children) = query.get(entity) {
				for child in children.iter() {
					match first(*child, query, func) {
						None => {}
						some => return some,
					}
				}
			}
			None
		}
		first(entity, query, &mut func)
	}

	pub fn visit_dfs_world(
		world: &mut World,
		entity: Entity,
		mut func: impl FnMut(&mut World, Entity),
	) {
		let mut query = world.query::<&Children>();
		Self::visit_dfs_recursive_world(
			world,
			&mut query,
			entity,
			&mut func,
			&mut HashSet::default(),
		);
	}

	fn visit_dfs_recursive_world(
		world: &mut World,
		edge_query: &mut QueryState<&Children>,
		entity: Entity,
		func: &mut impl FnMut(&mut World, Entity),
		visited: &mut HashSet<Entity>,
	) {
		if visited.contains(&entity) {
			return;
		}
		visited.insert(entity);
		func(world, entity);
		if let Ok(children) = edge_query.get(world, entity) {
			for child in
				children.into_iter().map(|edge| *edge).collect::<Vec<_>>()
			{
				Self::visit_dfs_recursive_world(
					world, edge_query, child, func, visited,
				);
			}
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

		let child1 = app.world_mut().spawn(Score::Fail).id();


		let _parent = app.world_mut().spawn_empty().add_child(child1);
		app.add_systems(Update, changes_score_to_pass);

		expect(&app)
			.component::<Score>(child1)?
			.to_be(&Score::Fail)?;
		app.update();
		expect(&app)
			.component::<Score>(child1)?
			.to_be(&Score::Pass)?;



		Ok(())
	}


	fn changes_score_to_pass(
		parents: Query<&Children>,
		mut scores: Query<&mut Score>,
	) {
		for children in parents.iter() {
			for child in children.iter() {
				if let Ok(mut score) = scores.get_mut(*child) {
					*score = Score::Pass;
				}
			}
		}
	}
}
