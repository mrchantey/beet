use crate::prelude::*;
use beet_core::prelude::*;


#[derive(SystemParam)]
pub struct ContextCommands<'w, 's> {
	commands: Commands<'w, 's>,
	query: ContextQuery<'w, 's>,
}
impl ContextCommands<'_, '_> {
	pub fn reborrow(&mut self) -> ContextCommands<'_, '_> {
		ContextCommands {
			commands: self.commands.reborrow(),
			query: self.query.reborrow(),
		}
	}

	pub fn actor(&mut self, id: ActorId) -> ActorCommands<'_, '_> {
		ActorCommands {
			id,
			query: self.query.reborrow(),
			commands: self.commands.reborrow(),
		}
	}
}

pub struct ActorCommands<'w, 's> {
	id: ActorId,
	query: ContextQuery<'w, 's>,
	commands: Commands<'w, 's>,
}
impl<'w, 's> ActorCommands<'w, 's> {
	pub fn id(&self) -> ActorId { self.id }

	pub fn add_item(
		&mut self,
		content: Content,
		scope: ItemScope,
	) -> Result<&mut Self> {
		let item = Item::new(self.id, content, scope);
		self.query.actor_mut(self.id)?.push(item.id());
		self.commands.spawn(item);
		self.xok()
	}
}
