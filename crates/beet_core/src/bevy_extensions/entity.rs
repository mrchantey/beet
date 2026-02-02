//! Extension methods for Bevy's [`EntityWorldMut`].

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
}
