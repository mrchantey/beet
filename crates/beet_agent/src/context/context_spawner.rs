use crate::prelude::*;
use beet_core::prelude::*;

/// Spawns context entities from a non-streaming response.
///
/// The `action` entity is stored in `ContextMeta::owner` for each
/// context entity, enabling proper role determination in multi-agent conversations.
pub async fn spawn_response_context(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	response: openresponses::ResponseBody,
) -> Result {
	world
		.with_then(move |world| {
			for item in &response.output {
				match item {
					openresponses::OutputItem::Message(item) => {
						spawn_response_item(
							world,
							agent,
							action,
							TextContext::from(item),
						);
					}
					openresponses::OutputItem::Reasoning(item) => {
						spawn_response_item(
							world,
							agent,
							action,
							ReasoningContext::from(item),
						);
					}
					openresponses::OutputItem::FunctionCall(item) => {
						spawn_response_item(
							world,
							agent,
							action,
							FunctionCallContext::from(item),
						);
					}
					openresponses::OutputItem::FunctionCallOutput(item) => {
						spawn_response_item(
							world,
							agent,
							action,
							FunctionOutputContext::from(item),
						);
					}
				}
			}
		})
		.await;
	Ok(())
}

fn spawn_response_item(
	world: &mut World,
	agent: Entity,
	action: Entity,
	item: impl Bundle,
) -> Entity {
	world
		.spawn((
			ThreadContextOf(agent),
			OwnedContextOf(action),
			ContextRole::Assistant,
			ContextComplete,
			item,
		))
		.id()
}


/// Spawns a user context with the given text.
pub fn spawn_user_text(
	commands: &mut Commands,
	agent: Entity,
	action: Entity,
	text: impl Into<String>,
) -> Entity {
	spawn_user_context(commands, agent, action, TextContext::new(text))
}
/// Spawns a user context with the given text.
pub fn spawn_user_file(
	commands: &mut Commands,
	agent: Entity,
	action: Entity,
	path: WsPathBuf,
) -> Entity {
	spawn_user_context(commands, agent, action, FileContext::from_fs(path))
}

fn spawn_user_context(
	commands: &mut Commands,
	agent: Entity,
	action: Entity,
	bundle: impl Bundle,
) -> Entity {
	commands
		.spawn((
			ThreadContextOf(agent),
			OwnedContextOf(action),
			ContextRole::User,
			ContextComplete,
			bundle,
		))
		.id()
}



/// Spawns a system context with the given text.
///
/// The `action` entity is stored in `ContextMeta::owner` for tracking.
pub async fn spawn_system_text(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	text: impl Into<String>,
) -> Entity {
	world
		.spawn_then((
			ThreadContextOf(agent),
			OwnedContextOf(action),
			TextContext::new(text),
			ContextRole::System,
			ContextComplete,
		))
		.await
		.id()
}
