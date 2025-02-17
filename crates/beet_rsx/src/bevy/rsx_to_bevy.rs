use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

#[derive(Debug, Default)]
pub struct RsxToBevy {
	pub rsx_idx_map: RsxIdxMap,
	rsx_idx_incr: RsxIdxIncr,
}


impl RsxToBevy {
	pub fn spawn_node(
		&mut self,
		world: &mut World,
		node: impl AsRef<RsxNode>,
	) -> Result<Vec<Entity>> {
		let idx = self.rsx_idx_incr.next();
		let nodes = match node.as_ref() {
			RsxNode::Doctype => unimplemented!(),
			RsxNode::Comment(_) => {
				unimplemented!()
			}
			RsxNode::Text(str) => {
				#[cfg(feature = "bevy_ui")]
				{
					let entity = world.spawn(Text::new(str)).id();
					vec![entity]
				}
				#[cfg(not(feature = "bevy_ui"))]
				{
					unimplemented!(
						"cannot add {str},add feature bevy_ui to enable"
					)
				}
			}
			RsxNode::Fragment(rsx_nodes) => rsx_nodes
				.iter()
				.map(|n| self.spawn_node(world, n))
				.collect::<Result<Vec<_>>>()?
				.into_iter()
				.flatten()
				.collect(),
			RsxNode::Block(rsx_block) => {
				self.spawn_node(world, &rsx_block.initial)?
			}
			RsxNode::Element(element) => {
				vec![self.spawn_element(world, idx, element)?]
			}
			RsxNode::Component(RsxComponent {
				tag: _,
				tracker: _,
				root,
				slot_children,
			}) => {
				slot_children.assert_empty();
				self.spawn_node(world, root.as_ref())?
			}
		};
		Ok(nodes)
	}
	fn spawn_element(
		&mut self,
		world: &mut World,
		idx: RsxIdx,
		element: &RsxElement,
	) -> Result<Entity> {
		// Arc::clone
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();

		let children = self.spawn_node(world, &element.children)?;

		let mut entity = world.spawn(BevyRsxElement {
			tag: element.tag.clone(),
			idx,
		});
		entity.add_children(&children);

		// println!("here");
		for attr in element.attributes.iter() {
			self.spawn_bevy_components(&registry, &mut entity, attr)?;
		}

		Ok(entity.id())
	}
	// #[allow(unused)]
	fn spawn_bevy_components(
		&mut self,
		registry: &TypeRegistry,
		entity: &mut EntityWorldMut,
		attr: &RsxAttribute,
	) -> Result<()> {
		match attr {
			RsxAttribute::Key { key } => {
				let (reflect_default, reflect_component) =
					ReflectUtils::reflect_component(key, registry)?;
				let default = reflect_default.default();
				// how to cast?
				// if reflect_component.contains(entity) {
				// 	return Ok(());
				// }
				reflect_component.insert(
					entity,
					default.as_partial_reflect(),
					registry,
				);
			}
			RsxAttribute::KeyValue { key, value } => {
				ReflectUtils::apply_or_insert_at_path(
					registry, entity, key, value,
				)?;
			}
			#[allow(unused)]
			RsxAttribute::BlockValue {
				key,
				initial,
				effect,
			} => {
				ReflectUtils::apply_or_insert_at_path(
					registry, entity, key, initial,
				)?;
			}
			RsxAttribute::Block { initial, effect: _ } => {
				for attr in initial.iter() {
					self.spawn_bevy_components(registry, entity, attr)?;
				}
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn attribute_key() {
		let mut app = App::new();
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>();

		let node = rsx! {<entity Transform/>};
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), node)
			.unwrap()[0];


		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::default()));
	}
	#[test]
	fn attribute_key_value() {
		let mut app = App::new();
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>();

		let node = rsx! {<entity Transform.translation="(0.,1.,2.)"/>};
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), node)
			.unwrap()[0];

		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::from_xyz(0., 1., 2.)));
	}
	#[test]
	// #[ignore = "requires multiple runtimes"]
	fn attribute_block_value() {
		// without the runtime registration it will still serialize
		// but with the wrong vec3 format, ie:
		// (x:0.0,y:1.0,z:2.0) instead of the custom glam serde
		// of (0.,1.,2.)
		BevyRuntime::with(|app| {
			app.register_type::<Transform>();
		});

		let mut app = App::new();
		let val = Vec3::new(0., 1., 2.);
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Vec3>()
			.register_type::<Transform>();

		let node = rsx! {<entity runtime:bevy Transform.translation={val}/>};
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), node)
			.unwrap()[0];

		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::from_xyz(0., 1., 2.)));
	}
}
