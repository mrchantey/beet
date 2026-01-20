use beet_core::prelude::*;
use beet_flow::prelude::AgentQuery;


/// A piece of context belonging to a flow agent, and possibly
/// shared by multiple AI agents.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Context)]
pub struct ContextOf(pub Entity);


/// The Flow Agent containing an ordered collection of context.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ContextOf, linked_spawn)]
pub struct Context(Vec<Entity>);


#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct TextContext(pub String);


#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub contexts: AgentQuery<'w, 's, &'static Context>,
	pub text_contexts: Query<'w, 's, &'static TextContext>,
}


impl<'w, 's> ContextQuery<'w, 's> {
	/// Get the text contexts for a given flow agent
	pub fn texts(&self, action: Entity) -> Vec<&TextContext> {
		let mut texts = Vec::new();
		if let Ok(context) = self.contexts.get(action) {
			for ctx_entity in context.iter() {
				if let Ok(text_ctx) = self.text_contexts.get(ctx_entity) {
					texts.push(text_ctx);
				}
			}
		}
		texts
	}
}
