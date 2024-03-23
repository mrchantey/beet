#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
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
	fn works() -> Result<()> {
		let mut registry = TypeRegistry::default();
		// registry.register_type_data::<ReflectInspectorOptions>();
		registry.register::<MyStruct>();
		let my_struct = MyStruct { field: 5 };
		let reflect_do_thing = registry
			.get_type_data::<ReflectInspectorOptions>(my_struct.type_id())
			.unwrap();
		let (_key, val) = reflect_do_thing.0.iter().next().unwrap();
		let num_opts = val.downcast_ref::<NumberOptions<u8>>().unwrap();
		expect(num_opts.max).to_be(Some(10))?;

		Ok(())
	}
}
