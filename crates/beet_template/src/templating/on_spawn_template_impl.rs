use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;


/// Add this system for [`OnSpawnTemplate`] behavior.
/// It must be called after *apply_slots* as it doesnt recurse into [`TemplateOf`]
pub fn on_spawn_template(
	mut commands: Commands,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	attributes_of: Query<&AttributeOf>,
	mut query: Populated<
		(Entity, &mut OnSpawnTemplate),
		Added<OnSpawnTemplate>,
	>,
) -> Result {
	let roots: HashSet<Entity> = query
		.iter()
		.map(|(entity, _)| {
			if let Ok(attribute_of) = attributes_of.get(entity) {
				parents.root_ancestor(attribute_of.entity())
			} else {
				parents.root_ancestor(entity)
			}
		})
		.collect();

	for entity in roots
		.into_iter()
		.flat_map(|root| children.iter_descendants_inclusive(root))
	{
		if let Ok((entity, mut on_spawn)) = query.get_mut(entity) {
			commands.entity(entity).remove::<OnSpawnTemplate>();
			on_spawn.take().call(commands.entity(entity))?;
		}
		if let Ok(attrs) = attributes.get(entity) {
			for attr in attrs.iter() {
				if let Ok((attr_entity, mut on_spawn)) = query.get_mut(attr) {
					commands.entity(attr_entity).remove::<OnSpawnTemplate>();
					on_spawn.take().call(commands.entity(attr_entity))?;
				}
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
		world.spawn((
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
		));
		world.run_system_once(on_spawn_template).unwrap().unwrap();
		expect(val()).to_be(vec![0, 1, 2, 3, 4]);
	}
}
