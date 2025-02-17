use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::serde::TypedReflectDeserializer;
use bevy::reflect::DynamicStruct;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistry;
use serde::de::DeserializeSeed;

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
				let entity = world.spawn(Text::new(str)).id();
				vec![entity]
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
		_idx: RsxIdx,
		element: &RsxElement,
	) -> Result<Entity> {
		// Arc::clone
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();

		let children = self.spawn_node(world, &element.children)?;

		let mut entity = world.spawn(ElementTag {
			tag: element.tag.clone(),
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
					parse_attribute_key(key, registry)?;
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
			#[allow(unused)]
			RsxAttribute::KeyValue { key, value } => {
				let mut parts = key.split('.');
				let key = parts.next().unwrap();
				let field_path = parts.collect::<Vec<_>>().join(".");
				let (reflect_default, reflect_component) =
					parse_attribute_key(key, registry)?;
				if let Some(mut target) =
					reflect_component.reflect_mut(&mut *entity)
				{
					apply_reflect(
						registry,
						&field_path,
						target.as_reflect_mut(),
						value,
					)?;
				} else {
					let mut default = reflect_default.default();
					apply_reflect(
						registry,
						&field_path,
						default.as_mut(),
						value,
					)?;
					reflect_component.insert(
						entity,
						default.as_partial_reflect(),
						registry,
					);
				}
			}
			#[allow(unused)]
			RsxAttribute::BlockValue {
				key,
				initial,
				effect,
			} => {
				println!("initial: {:?}", initial);
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

/// Path may be empty, in which case the value is applied to the type itself.
fn apply_reflect(
	registry: &TypeRegistry,
	path: &str,
	target: &mut dyn Reflect,
	value: &str,
) -> Result<()> {
	match target.reflect_type_info() {
		TypeInfo::Struct(info) => {
			if path.is_empty() {
				todo!();
			} else {
				let field = info
					.field(path)
					.ok_or_else(|| {
						anyhow::anyhow!(
							"Could not find field {} in struct",
							path
						)
					})?
					.type_info()
					.unwrap();
				let registration =
					registry.get(field.type_id()).ok_or_else(|| {
						anyhow::anyhow!(
							"Could not find registration for field type"
						)
					})?;
				let reflect_deserializer =
					TypedReflectDeserializer::new(registration, registry);
				let mut deserializer = ron::de::Deserializer::from_str(&value)?;
				let reflect_value =
					reflect_deserializer.deserialize(&mut deserializer)?;
				let mut dyn_struct = DynamicStruct::default();
				dyn_struct.insert_boxed(path, reflect_value);
				target.apply(&dyn_struct);
			}
		}
		_ => {
			todo!();
		}
	}
	Ok(())
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
	#[ignore = "requires multiple runtimes"]
	fn attribute_block_value() {
		let mut app = App::new();
		let val = Vec3::new(0., 1., 2.);
		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>();

		let node = rsx! {<entity runtime:bevy Transform.translation={val}/>};
		let entity = RsxToBevy::default()
			.spawn_node(app.world_mut(), node)
			.unwrap()[0];

		expect(app.world_mut().entity(entity).get::<Transform>())
			.to_be(Some(&Transform::from_xyz(0., 1., 2.)));
	}
}
