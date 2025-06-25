use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::prelude::*;


/// Add this system for [`OnSpawnTemplate`] behavior.
/// It must be called after *apply_slots* as it doesnt recurse into [`TemplateOf`]
pub fn on_spawn_template(
	In(root): In<Entity>,
	mut commands: Commands,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	mut query: Populated<
		(Entity, &mut OnSpawnTemplate),
		Added<OnSpawnTemplate>,
	>,
) -> Result {
	for entity in children.iter_descendants_inclusive(root) {
		if let Ok((entity, mut on_spawn)) = query.get_mut(entity) {
			commands.entity(entity).remove::<OnSpawnTemplate>();
			on_spawn.take().call(commands.entity(entity))?;
		}
		for attr in attributes.iter_direct_descendants(entity) {
			if let Ok((attr_entity, mut on_spawn)) = query.get_mut(attr) {
				commands.entity(attr_entity).remove::<OnSpawnTemplate>();
				on_spawn.take().call(commands.entity(attr_entity))?;
			}
		}
	}
	Ok(())
}
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (val, set_val) = signal::<Vec<u32>>(Vec::new());

		let mut world = World::new();
		let entity = world
			.spawn((
				OnSpawnTemplate::new(move |_| {
					set_val.update(|v| v.push(0));
					Ok(())
				}),
				children![
					(
						OnSpawnTemplate::new(move |_| {
							set_val.update(|v| v.push(1));
							Ok(())
						}),
						children![
							OnSpawnTemplate::new(move |_| {
								set_val.update(|v| v.push(3));
								Ok(())
							}),
							related! {Attributes[
								OnSpawnTemplate::new(move |_| {
									set_val.update(|v| v.push(4));
									Ok(())
								})
							]}
						],
					),
					// sibling, bfs!
					OnSpawnTemplate::new(move |_| {
						set_val.update(|v| v.push(2));
						Ok(())
					}),
				],
			))
			.id();
		world
			.run_system_once_with(on_spawn_template, entity)
			.unwrap()
			.unwrap();
		expect(val()).to_be(vec![0, 1, 2, 3, 4]);
	}
}
