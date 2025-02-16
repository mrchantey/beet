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
	pub fn spawn(
		&mut self,
		world: &mut World,
		node: impl AsRef<RsxNode>,
	) -> Result<()> {
		self.spawn_node(world, node);
		Ok(())
	}


	pub fn spawn_node(
		&mut self,
		world: &mut World,
		node: impl AsRef<RsxNode>,
	) -> Vec<Entity> {
		let idx = self.rsx_idx_incr.next();
		match node.as_ref() {
			RsxNode::Doctype => unimplemented!(),
			RsxNode::Comment(_) => {
				unimplemented!()
			}
			RsxNode::Text(str) => {
				let entity = world.spawn(Text::new(str)).id();
				vec![entity]
			}
			RsxNode::Fragment(rsx_nodes) => rsx_nodes
				.iter()
				.map(|n| self.spawn_node(world, n))
				.flatten()
				.collect(),
			RsxNode::Block(rsx_block) => {
				self.spawn_node(world, &rsx_block.initial)
			}
			RsxNode::Element(element) => {
				vec![self.spawn_element(world, idx, element)]
			}
			RsxNode::Component(RsxComponent {
				tag: _,
				tracker: _,
				root,
				slot_children,
			}) => {
				slot_children.assert_empty();
				self.spawn_node(world, root.as_ref())
			}
		}
	}
	#[allow(unused)]
	fn spawn_element(
		&mut self,
		world: &mut World,
		idx: RsxIdx,
		element: &RsxElement,
	) -> Entity {
		// Arc::clone
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();

		let children = self.spawn_node(world, &element.children);

		let mut entity = world.spawn(ElementTag {
			tag: element.tag.clone(),
		});
		entity.add_children(&children);

		for attr in element.attributes.iter() {
			self.spawn_bevy_components(&registry, &mut entity, attr);
		}

		entity.id()
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
					parse_attribute_key(key, registry)?;
				let default = reflect_default.default();
				reflect_component.insert(
					entity,
					default.as_partial_reflect(),
					registry,
				);
			}
			#[allow(unused)]
			RsxAttribute::KeyValue { key, value } => {
				let mut parts = key.split('.');
				let key = parts.next().unwrap();
				let (reflect_default, reflect_component) =
					parse_attribute_key(key, registry)?;
				todo!()
			}
			#[allow(unused)]
			RsxAttribute::BlockValue {
				key,
				initial,
				effect,
			} => {
				todo!()
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


fn parse_attribute_key<'a>(
	key: &str,
	registry: &'a TypeRegistry,
) -> Result<(&'a ReflectDefault, &'a ReflectComponent)> {
	let registration =
		registry.get_with_short_type_path(key).ok_or_else(|| {
			anyhow::anyhow!("Could not find short type path for key: {}", key)
		})?;
	let reflect_default =
		registration.data::<ReflectDefault>().ok_or_else(|| {
			anyhow::anyhow!("Could not find reflect default for key: {}", key)
		})?;
	let reflect_component =
		registration.data::<ReflectComponent>().ok_or_else(|| {
			anyhow::anyhow!("Could not find reflect component for key: {}", key)
		})?;

	Ok((reflect_default, reflect_component))
}
