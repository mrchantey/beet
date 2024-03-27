use bevy::prelude::*;
use std::any::TypeId;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ComponentType {
	pub name: String,
	pub type_name: String,
	pub type_id: TypeId,
}
impl ComponentType {
	pub fn new(type_name: String, type_id: TypeId) -> Self {
		let name = heck::AsTitleCase(type_name.clone()).to_string();
		Self {
			name,
			type_name,
			type_id,
		}
	}

	pub fn from_world(world: &World) -> Vec<Self> {
		let registry = world.resource::<AppTypeRegistry>().read();

		world
			.components()
			.iter()
			.filter_map(|info| registry.get_type_info(info.type_id().unwrap()))
			.filter(|info| {
				let Some(registration) = registry.get(info.type_id()) else {
					return false;
				};
				if let Some(_) = registration.data::<ReflectDefault>() {
					true
				} else {
					false
				}
			})
			.map(|info| {
				ComponentType::new(
					info.type_path_table().short_path().to_string(),
					// info.type_path_table().ident().unwrap().to_string(),
					info.type_id(),
				)
			})
			.collect()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use std::any::TypeId;
	use sweet::*;

	#[test]
	fn component_types() -> Result<()> {
		pretty_env_logger::try_init().ok();

		// Create a world and an entity
		let graph = (EmptyAction.child((EmptyAction, SetOnRun(Score::Pass))))
			.into_graph::<EcsNode>();

		let types = graph.component_types();
		expect(types.len()).to_be_greater_than(0)?;

		// for ty in types.iter() {
		// 	log::info!("Type: {:?}", ty);
		// }

		expect(types.contains(&ComponentType {
			name: "Empty Action".to_string(),
			type_name: "EmptyAction".to_string(),
			type_id: TypeId::of::<EmptyAction>(),
		}))
		.to_be_true()?;

		Ok(())
	}
}
