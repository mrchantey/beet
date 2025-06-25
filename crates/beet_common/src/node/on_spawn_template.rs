use bevy::prelude::*;

/// A method that will run *after* spawned, in breadth-first order from
/// its root, meaning that parents are guaranteed to run before children.
#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct OnSpawnTemplate(
	Box<dyn 'static + Send + Sync + FnOnce(EntityCommands) -> Result>,
);

impl OnSpawnTemplate {
	/// Create a new [`OnSpawnTemplate`] effect.
	pub fn new(
		func: impl 'static + Send + Sync + FnOnce(EntityCommands) -> Result,
	) -> Self {
		Self(Box::new(func))
	}

	/// Insert this bundle into the entity on spawn
	pub fn new_insert(bundle: impl Bundle) -> Self {
		Self::new(move |mut entity: EntityCommands| {
			entity.insert(bundle);
			Ok(())
		})
	}


	/// Convenience for getting the method from inside a system,
	/// this component should be removed when this is called
	///
	/// # Panics
	/// If the method has already been taken and is called, this will panic.
	pub fn take(&mut self) -> Self {
		Self::new(std::mem::replace(
			&mut self.0,
			Box::new(|_| {
				panic!(
					"OnSpawnTemplate: This method has been taken and not removed"
				)
			}),
		))
	}
	pub fn call(self, entity: EntityCommands) -> Result { (self.0)(entity) }
}



// implemented and tested in beet_template
