use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::Access;
use bevy::reflect::TypeData;

impl FieldIdent {
	pub fn inspector_target(
		&self,
		variant_index: Option<usize>,
	) -> Option<InspectorTarget> {
		let Some(access) = self.path.last() else {
			return None;
		};
		// InspectorTarget does not discrimiate between tuple and struct index
		let field_index = match access {
			Access::FieldIndex(index) => *index,
			Access::TupleIndex(index) => *index,
			_ => return None, // named field or list index not supported
		};

		let inspector_target = if let Some(variant_index) = variant_index {
			InspectorTarget::VariantField {
				variant_index,
				field_index,
			}
		} else {
			InspectorTarget::Field(field_index)
		};
		Some(inspector_target)
	}

	pub fn inspector_options<T: TypeData>(&self, world: &World) -> Result<T> {
		let Some(parent) = self.parent() else {
			anyhow::bail!("field has no parent")
		};

		let variant_index = parent.variant_index(world).ok();

		let Some(target) = self.inspector_target(variant_index) else {
			anyhow::bail!("failed to get inspector target from field")
		};

		let registry = world.resource::<AppTypeRegistry>();
		let result: Result<T> = parent
			.map(world, move |parent| {
				let registry = registry.read();
				let type_info = parent
					.get_represented_type_info()
					.ok_or_else(|| anyhow::anyhow!("field has no type info"))?;

				let type_path = type_info.type_path();

				let inspector_opts = registry
					.get_type_data::<ReflectInspectorOptions>(
						type_info.type_id(),
					)
					.ok_or_else(|| {
						anyhow::anyhow!(
							"{type_path} is not ReflectInspectorOptions"
						)
					})?;
				let val =
					inspector_opts.0.get_cloned(target).ok_or_else(|| {
						anyhow::anyhow!(
				"{type_path} has no options for this InspectorTarget"
			)
					})?;
				val.downcast::<T>()
					.map(|val| *val)
					.map_err(|_| anyhow::anyhow!("failed to downcast"))
			})
			.flatten();

		match result {
			Ok(result) => Ok(result),
			Err(err) => match self.parent() {
				// search parents recursively
				Some(parent) => parent.inspector_options::<T>(world),
				None => Err(err),
			},
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::reflect::Access;
	use std::any::TypeId;
	use sweet::*;

	#[derive(
		Debug, Default, PartialEq, Component, Reflect, InspectorOptions,
	)]
	#[reflect(Default, Component, InspectorOptions)]
	struct MyVecStruct(#[inspector(min = 2.)] pub Vec3);

	#[test]
	fn inspector_options() -> Result<()> {
		// setup
		pretty_env_logger::try_init().ok();
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource::<AppTypeRegistry>();
		registry.write().register::<MyVecStruct>();

		let entity = world.spawn(MyVecStruct(Vec3::default())).id();

		let field = FieldIdent::new_with_entity(
			entity,
			TypeId::of::<MyVecStruct>(),
			vec![Access::TupleIndex(0)],
		);

		let options = field.inspector_options::<NumberOptions<f32>>(&world)?;
		expect(options.min).as_some()?.to_be(2.)?;

		let nested_field = field.child(Access::FieldIndex(2));
		let options =
			nested_field.inspector_options::<NumberOptions<f32>>(&world)?;
		expect(options.min).as_some()?.to_be(2.)?;

		Ok(())
	}
}
