use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

pub trait BeetModule: 'static + Send + Sync + ActionSystems {
	/// Register components via [`World::init_bundle`]
	fn register_bundles(world: &mut World);
	/// Register types via [`TypeRegistry::register`]
	fn register_types(type_registry: &mut TypeRegistry);

	/// Create an [`AppTypeRegistry`] with all types in this module registered
	fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		Self::register_types(&mut registry.write());
		registry
	}
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use anyhow::Result;
	// use bevy::prelude::*;
	// use bevy::reflect::TypeRegistry;
	// use sweet::*;

	#[test]
	fn works() -> Result<()> {
		// let mut world = World::new();
		// world.init_bundle::<TransformBundle>();
		// let foo = world.resource::<AppTypeRegistry>();

		// let registry = TypeRegistry::default();
		// registry.register::<TransformBundle>();
		// expect(registry.registrations().len()).to_be(1)?;

		// expect(true).to_be_false()?;

		Ok(())
	}
}
