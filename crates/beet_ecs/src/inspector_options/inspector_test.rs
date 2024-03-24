#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::reflect::Enum;
	use bevy::reflect::TypeRegistry;
	use std::any::Any;
	use sweet::*;

	#[derive(Debug, PartialEq, InspectorOptions, Reflect)]
	#[reflect(InspectorOptions)]
	struct MyStruct {
		#[inspector(min = 0, max = 10, step = 2)]
		pub field: u8,
	}



	#[test]
	fn works_struct() -> Result<()> {
		let mut registry = TypeRegistry::default();
		// registry.register_type_data::<ReflectInspectorOptions>();
		registry.register::<MyStruct>();
		let my_val = MyStruct { field: 5 };
		let inspector_opts = registry
			.get_type_data::<ReflectInspectorOptions>(my_val.type_id())
			.unwrap();

		let (_key, val) = inspector_opts.0.iter().next().unwrap();
		let num_opts = val.downcast_ref::<NumberOptions<u8>>().unwrap();
		expect(num_opts.max).to_be(Some(10))?;

		Ok(())
	}

	#[derive(Debug, PartialEq, InspectorOptions, Reflect)]
	#[reflect(InspectorOptions)]
	enum MyEnum {
		Foo,

		Bar(#[inspector(min = 0, max = 10, step = 2)] u8),
		Bazz {
			#[inspector(min = 0, max = 10, step = 2)]
			val: u8,
		},
	}

	#[test]
	fn works_enum() -> Result<()> {
		let mut registry = TypeRegistry::default();
		// registry.register_type_data::<ReflectInspectorOptions>();
		registry.register::<MyEnum>();
		let my_val = MyEnum::Bar(30);
		let inspector_opts = registry
			.get_type_data::<ReflectInspectorOptions>(my_val.type_id())
			.unwrap();

		expect(inspector_opts.0.get(InspectorTarget::VariantField {
			variant_index: 2,
			field_index: 0,
		}))
		.to_be_some()?;



		let variant_index = my_val.variant_index();
		let field_index = 0;


		expect(inspector_opts.0.get(InspectorTarget::VariantField {
			variant_index,
			field_index: 1,
		}))
		.to_be_none()?;

		let val = inspector_opts
			.0
			.get(InspectorTarget::VariantField {
				variant_index,
				field_index,
			})
			.unwrap();
		let num_opts = val.downcast_ref::<NumberOptions<u8>>().unwrap();
		expect(num_opts.max).to_be(Some(10))?;

		Ok(())
	}

	#[derive(Debug, PartialEq, InspectorOptions, Reflect)]
	#[reflect(InspectorOptions)]
	struct MyVec3 {
		#[inspector(min = 0., max = 10., step = 2.)]
		pub field: Vec3,
	}



	#[test]
	fn works_vec3() -> Result<()> {
		let mut registry = TypeRegistry::default();
		// registry.register_type_data::<ReflectInspectorOptions>();
		registry.register::<MyVec3>();
		let my_val = MyVec3 {
			field: Vec3::default(),
		};
		let inspector_opts = registry
			.get_type_data::<ReflectInspectorOptions>(my_val.type_id())
			.unwrap();

		let (_key, val) = inspector_opts.0.iter().next().unwrap();
		let num_opts = val.downcast_ref::<NumberOptions<f32>>().unwrap();
		expect(num_opts.max).to_be(Some(10.))?;

		Ok(())
	}
}
