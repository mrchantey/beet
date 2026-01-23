use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::AgentQuery;

/// A view of a single context item for iteration.
#[allow(unused)]
#[derive(Debug)]
enum ContextView<'a> {
	Text(&'a TextContext),
	Reasoning(&'a ReasoningContext),
	File(&'a FileContext),
	FunctionCall(&'a FunctionCallContext),
	FunctionOutput(&'a FunctionOutputContext),
}


/// System parameter for querying context entities.
#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub contexts: AgentQuery<'w, 's, &'static ThreadContext>,
	pub text_contexts: Query<'w, 's, &'static TextContext>,
	pub reasoning_contexts: Query<'w, 's, &'static ReasoningContext>,
	pub file_contexts: Query<'w, 's, &'static FileContext>,
	pub function_calls: Query<'w, 's, &'static FunctionCallContext>,
	pub function_outputs: Query<'w, 's, &'static FunctionOutputContext>,
	pub roles: Query<'w, 's, &'static ContextRole>,
	pub context_owners: Query<'w, 's, &'static OwnedContextOf>,
	pub names: Query<'w, 's, &'static Name>,
}


impl<'w, 's> ContextQuery<'w, 's> {
	/// Convert context entities to openresponses input items, filtering by predicate.
	///
	/// This collects all context and converts it to the format expected by
	/// the OpenResponses API. Role determination in multi-agent conversations:
	/// - Context created by `action` (the current action) → Assistant role
	/// - Context created by other actions → User role, prefixed with creator's name
	///
	/// The `action` parameter identifies who "I" am in this conversation.
	pub fn collect_input_items(
		&self,
		action: Entity,
		filter: impl Fn(Entity) -> bool,
	) -> Result<Vec<openresponses::request::InputItem>> {
		let mut items = Vec::new();

		if let Ok(context) = self.contexts.get(action) {
			for ctx_entity in context.iter().filter(|e| filter(*e)) {
				// Determine role based on who created this context
				let cx_owner =
					self.context_owners.get(ctx_entity).ok().map(|m| **m);

				// "I created it" → Assistant, "someone else" → User
				let effective_role = if cx_owner == Some(action) {
					ContextRole::Assistant
				} else {
					// Use the stored role, but treat other assistants as users
					let stored_role = self
						.roles
						.get(ctx_entity)
						.copied()
						.unwrap_or(ContextRole::User);
					match stored_role {
						ContextRole::Assistant => ContextRole::User,
						other => other,
					}
				};

				// Get the creator's name for prefixing (only for non-self contexts)
				let creator_name = if cx_owner != Some(action) {
					cx_owner.and_then(|entity| {
						self.names
							.get(entity)
							.ok()
							.map(|n| n.as_str().to_string())
					})
				} else {
					None
				};

				// Build message content parts
				let mut parts = Vec::new();

				if let Ok(text) = self.text_contexts.get(ctx_entity) {
					// Prefix with creator's name if from another agent
					let text_content = if let Some(name) = &creator_name {
						format!("{} > {}", name, text.0)
					} else {
						text.0.clone()
					};
					parts.push(openresponses::ContentPart::input_text(
						&text_content,
					));
				}

				if let Ok(file) = self.file_contexts.get(ctx_entity) {
					parts.push(file.to_content_part(
						effective_role != ContextRole::Assistant,
					));
				}

				if let Ok(reasoning) = self.reasoning_contexts.get(ctx_entity) {
					// Reasoning is typically from assistant
					let reasoning_content = if let Some(name) = &creator_name {
						format!("{} > {}", name, reasoning.0)
					} else {
						reasoning.0.clone()
					};
					parts.push(openresponses::ContentPart::InputText(
						openresponses::InputText::new(&reasoning_content),
					));
				}

				// Handle function calls
				if let Ok(func_call) = self.function_calls.get(ctx_entity) {
					items
						.push(openresponses::request::InputItem::FunctionCall(
						openresponses::request::FunctionCallParam {
							id: None,
							call_id: func_call.call_id.clone(),
							name: func_call.name.clone(),
							arguments: func_call.arguments.clone(),
							status: Some(
								// TODO this is a lie, status may not be complete
								openresponses::FunctionCallStatus::Completed,
							),
						},
					));
					continue;
				}

				// Handle function outputs
				if let Ok(func_output) = self.function_outputs.get(ctx_entity) {
					items.push(openresponses::request::InputItem::FunctionCallOutput(
						openresponses::request::FunctionCallOutputParam::text(
							&func_output.call_id,
							&func_output.output,
						),
					));
					continue;
				}

				// Skip empty content
				if parts.is_empty() {
					continue;
				}

				// Create message with effective role
				let message =
					if parts.len() == 1 {
						if let Some(text) = parts[0].as_text() {
							match effective_role {
							ContextRole::User => {
								openresponses::request::MessageParam::user(text)
							}
							ContextRole::Assistant => {
								openresponses::request::MessageParam::assistant(text)
							}
							ContextRole::System => {
								openresponses::request::MessageParam::system(text)
							}
							ContextRole::Developer => {
								openresponses::request::MessageParam::developer(text)
							}
						}
						} else {
							openresponses::request::MessageParam {
							id: None,
							role: effective_role.to_message_role(),
							content: openresponses::request::MessageContent::Parts(parts),
							status: None,
						}
						}
					} else {
						openresponses::request::MessageParam {
							id: None,
							role: effective_role.to_message_role(),
							content:
								openresponses::request::MessageContent::Parts(
									parts,
								),
							status: None,
						}
					};

				items.push(openresponses::request::InputItem::Message(message));
			}
		}

		items.xok()
	}
}
