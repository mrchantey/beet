use bevy::prelude::*;


pub struct ParentExt;

impl ParentExt {
	pub fn visit(
		entity: Entity,
		parents: &Query<&Parent>,
		mut func: impl FnMut(Entity),
	) {
		func(entity);
		if let Ok(parent) = parents.get(entity) {
			Self::visit(**parent, parents, func);
		}
	}

	pub fn find<T>(
		entity: Entity,
		parents: &Query<&Parent>,
		mut func: impl FnMut(Entity) -> Option<T>,
	) -> Option<T> {
		if let Some(val) = func(entity) {
			return Some(val);
		}
		if let Ok(parent) = parents.get(entity) {
			Self::find(**parent, parents, func)
		} else {
			None
		}
	}


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
		app.add_systems(Update, set_root_as_target_agent);

		let world = app.world_mut();
		let grandparent = world.spawn(RootIsTargetAgent).id();
		let parent =
			world.spawn(RootIsTargetAgent).set_parent(grandparent).id();
		let child = world.spawn(RootIsTargetAgent).set_parent(parent).id();


		app.update();

		expect(app.world())
			.not()
			.to_have_component::<TargetAgent>(grandparent)?;
		expect(app.world())
			.component(parent)?
			.to_be(&TargetAgent(grandparent))?;
		expect(app.world())
			.component(child)?
			.to_be(&TargetAgent(grandparent))?;

		Ok(())
	}
}
