use crate::prelude::*;


#[extend::ext(name=CommandsExt)]
pub impl Commands<'_, '_> {
	fn log_component_names(&mut self, entity: Entity) {
		self.queue(move |world: &mut World| {
			world.log_component_names(entity);
		});
	}
}
