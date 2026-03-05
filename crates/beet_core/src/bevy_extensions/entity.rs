//! Extension methods for Bevy's [`EntityWorldMut`].

use bevy::ecs::component::Mutable;

use crate::prelude::*;

/// Extension trait adding utility methods to [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutExt)]
pub impl EntityWorldMut<'_> {
	/// Logs the names of all components on this entity.
	fn log_component_names(&mut self) -> &mut Self {
		let id = self.id();
		self.world_scope(|world| {
			world.log_component_names(id);
		});
		self
	}
	/// Sets the value of a component if it is different from the existing value, or inserts it if not present.
	fn set_if_ne_or_insert<T: Component<Mutability = Mutable> + PartialEq>(
		&mut self,
		value: T,
	) {
		match self.get_mut::<T>() {
			Some(mut existing) => {
				existing.set_if_neq(value);
			}
			None => {
				self.insert(value);
			}
		}
	}
}
