use beet_core::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::prelude::*;


/// Add this system for [`OnSpawnTemplate`] behavior.
/// It must be called after *apply_slots* as it doesnt recurse into [`TemplateOf`]
pub fn apply_on_spawn_template(
	mut commands: Commands,
	roots: Populated<Entity, Added<InstanceRoot>>,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	event_attrs: Query<&AttributeKey, Without<AttributeLit>>,
	mut query: Query<(Entity, &mut OnSpawnTemplate), Added<OnSpawnTemplate>>,
) -> Result {
	for root in roots.iter() {
		for entity in children.iter_descendants_inclusive(root) {
			if let Ok((entity, mut on_spawn)) = query.get_mut(entity) {
				// println!("Running onspawn for block node");
				commands.entity(entity).remove::<OnSpawnTemplate>();
				on_spawn.take().call(commands.entity(entity))?;
			}
			// only elements, not templates, will have attributes
			for attr in attributes.iter_direct_descendants(entity) {
				if let Ok((attr_entity, mut on_spawn)) = query.get_mut(attr) {
					commands.entity(attr_entity).remove::<OnSpawnTemplate>();
					match event_attrs.get(attr_entity) {
						Ok(event_key) if event_key.starts_with("on") => {
							// event attributes are an EntityObserver which should
							// be applied to the element not the attribute entity
							on_spawn.take().call(commands.entity(entity))?;
						}
						_ => {
							on_spawn
								.take()
								.call(commands.entity(attr_entity))?;
						}
					}
				}
			}
		}
	}
	Ok(())
}
#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use beet_common::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn bfs_order() {
		let (val, set_val) = signal::<Vec<u32>>(Vec::new());

		let mut world = World::new();
		world.spawn((
			InstanceRoot,
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
		world
			.run_system_once(apply_on_spawn_template)
			.unwrap()
			.unwrap();
		expect(val()).to_be(vec![0, 1, 2, 3, 4]);
	}


	fn parse(instance: impl Bundle) -> String {
		let mut world = World::new();
		let instance = world.spawn(instance).id();

		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
		world.run_system_once(apply_slots).ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}

	#[test]
	fn templates() {
		#[template]
		fn MyTemplate() -> impl Bundle {
			rsx! {<div/>}
		}

		parse(rsx! {
			<MyTemplate/>
		})
		.xpect()
		.to_be_str("<div/>");
	}
	#[test]
	fn attribute_blocks() {
		#[derive(Default, Buildable, AttributeBlock)]
		struct MyAttributeBlock {
			class: String,
		}

		#[template]
		fn MyTemplate(
			#[field(flatten)] attrs: MyAttributeBlock,
		) -> impl Bundle {
			rsx! {<div {attrs}/>}
		}

		parse(rsx! {
			<MyTemplate class="foo"/>
		})
		.xpect()
		.to_be_str("<div class=\"foo\"/>");
	}
}
