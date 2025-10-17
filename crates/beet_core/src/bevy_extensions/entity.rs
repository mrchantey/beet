use crate::prelude::*;


#[extend::ext(name=EntityWorldMutExt)]
pub impl EntityWorldMut<'_> {
	fn log_component_names(&mut self) -> &mut Self {
		let id = self.id();
		self.world_scope(|world| {
			world.log_component_names(id);
		});
		self
	}
}
