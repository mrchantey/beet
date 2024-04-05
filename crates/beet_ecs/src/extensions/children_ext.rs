use bevy::prelude::*;
use bevy::utils::HashSet;

pub struct ChildrenExt;

impl ChildrenExt {
	pub fn collect(
		entity: Entity,
		edge_query: &Query<&Children>,
	) -> Vec<Entity> {
		let mut entities = Vec::new();
		Self::visit_dfs(entity, edge_query, |entity| {
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

	pub fn visit_dfs(
		entity: Entity,
		edge_query: &Query<&Children>,
		mut func: impl FnMut(Entity),
	) {
		Self::visit_dfs_recursive(
			entity,
			edge_query,
			&mut func,
			&mut HashSet::default(),
		);
	}

	fn visit_dfs_recursive(
		entity: Entity,
		edge_query: &Query<&Children>,
		func: &mut impl FnMut(Entity),
		visited: &mut HashSet<Entity>,
	) {
		if visited.contains(&entity) {
			return;
		}
		visited.insert(entity);
		func(entity);
		if let Ok(children) = edge_query.get(entity) {
			for child in children.iter() {
				Self::visit_dfs_recursive(*child, edge_query, func, visited);
			}
		}
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
