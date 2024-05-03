use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use std::marker::PhantomData;



#[derive(Default)]
pub struct BeetModulePlugin<T: BeetModule>(pub PhantomData<T>);

impl<T: BeetModule> Plugin for BeetModulePlugin<T> {
	fn build(&self, app: &mut App) {
		T::register_bundles(app.world_mut());
		T::register_types(
			&mut app.world().resource::<AppTypeRegistry>().write(),
		);
	}
}

/// Utility trait to assist registration of components & systems, can be recursive.
pub trait BeetModule: 'static + Send + Sync + ActionSystems {
	/// Register bundles/components via [`World::init_bundle`]
	fn register_bundles(world: &mut World);
	/// Register types via [`TypeRegistry::register`]
	fn register_types(type_registry: &mut TypeRegistry);

	/// Create an [`AppTypeRegistry`] with all types in this module registered
	fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		Self::register_types(&mut registry.write());
		registry
	}

	/// Get all ids registered to this module and any submodule
	fn infos() -> Vec<BeetTypeInfo>;
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
