use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;


/// RsxToBevy is different from RsxToHtml, its a lot simpler
/// because we dont need to deal with collapsing text nodes or
/// output a tree. Instead we can just use a visitor to spawn
/// nodes.
#[derive(Debug, Default)]
pub struct RsxToBevy {
	// we should probably use a tree visitor instead
	tree_idx_incr: TreeIdxIncr,
}


impl RsxToBevy {
	/// Registers effects and spawns the node
	pub fn spawn(node: RsxNode) -> Result<Vec<Entity>> {
		let entities = BevyRuntime::with_mut(|app| {
			Self::default().spawn_root(app.world_mut(), &node)
		})?;
		node.bpipe(RegisterEffects::default())?;
		Ok(entities)
	}

	pub fn spawn_root(
		&mut self,
		world: &mut World,
		root: &RsxNode,
	) -> Result<Vec<Entity>> {
		let entities = self.spawn_node(world, &root)?;
		// for entity in entities.iter() {
		// 	world
		// 		.entity_mut(*entity)
		// 		.insert(BevyRsxLocation::new(root.location.clone()));
		// }
		Ok(entities)
	}
	pub fn spawn_node(
		&mut self,
		world: &mut World,
		node: impl AsRef<RsxNode>,
	) -> Result<Vec<Entity>> {
		let tree_idx = self.tree_idx_incr.next();
		// println!("rsx_to_bevy found node: {:?}", node.as_ref().discriminant());
		let nodes = match node.as_ref() {
			RsxNode::Doctype(_) => unimplemented!(),
			RsxNode::Comment(_) => {
				unimplemented!()
			}
			RsxNode::Text(text) => {
				#[cfg(feature = "bevy_default")]
				{
					let entity =
						world.spawn((tree_idx, Text::new(&text.value))).id();
					vec![entity]
				}
				#[cfg(not(feature = "bevy_default"))]
				{
					unimplemented!(
						"currently cannot add text node without bevy_default\nvalue: {}",
						text.value
					)
				}
			}
			RsxNode::Fragment(fragment) => fragment
				.nodes
				.iter()
				.map(|n| self.spawn_node(world, n))
				.collect::<Result<Vec<_>>>()?
				.into_iter()
				.flatten()
				.collect(),
			RsxNode::Block(rsx_block) => {
				self.spawn_root(world, &rsx_block.initial)?
			}
			RsxNode::Element(element) => {
				vec![self.spawn_element(world, element, tree_idx)?]
			}
			RsxNode::Component(RsxComponent {
				root,
				slot_children,
				..
			}) => {
				slot_children.assert_empty();
				self.spawn_root(world, root)?
			}
		};
		Ok(nodes)
	}
	fn spawn_element(
		&mut self,
		world: &mut World,
		element: &RsxElement,
		tree_idx: TreeIdx,
	) -> Result<Entity> {
		// Arc::clone
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();

		let children = self.spawn_node(world, &element.children)?;

		let mut entity = world.spawn((tree_idx, BevyRsxElement {
			tag: element.tag.clone(),
		}));
		entity.add_children(&children);

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
				// events are registered by RegisterEffects
				if !key.starts_with("on") {
					ReflectUtils::apply_or_insert_at_path(
						registry, entity, key, initial,
					)?;
				}
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
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn attribute_key() {
		let mut app = App::new();
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>();

		let root = rsx! { <entity Transform /> };
		let entity = RsxToBevy::default()
			.spawn_root(app.world_mut(), &root)
			.unwrap()[0];


		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::default()));
		expect(app.world_mut().entity(entity).get::<TreeIdx>())
			.to_be(Some(&TreeIdx::new(1)));
	}
	#[test]
	fn attribute_key_value() {
		BevyRuntime::reset();
		let mut app = App::new();
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>();

		let root = rsx! { <entity Transform.translation="(0.,1.,2.)" /> };
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), &root)
			.unwrap()[0];

		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::from_xyz(0., 1., 2.)));
		expect(app.world_mut().entity(entity).get::<TreeIdx>())
			.to_be(Some(&TreeIdx::new(1)));
	}
	#[test]
	fn attribute_block_value() {
		BevyRuntime::reset();
		// without the runtime registration it will still serialize
		// but with the wrong vec3 format, ie:
		// (x:0.0,y:1.0,z:2.0) instead of the custom glam serde
		// of (0.,1.,2.)
		BevyRuntime::with_mut(|app| {
			app.register_type::<Transform>();
		});

		// here we can get away with using two apps, as long as they
		// both register transform
		let mut app = App::new();
		let val = Vec3::new(0., 1., 2.);
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Vec3>()
			.register_type::<Transform>();

		let root = rsx! { <entity runtime:bevy Transform.translation=val /> };
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), &root)
			.unwrap()[0];

		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::from_xyz(0., 1., 2.)));
		expect(app.world_mut().entity(entity).get::<TreeIdx>())
			.to_be(Some(&TreeIdx::new(1)));
	}
}
