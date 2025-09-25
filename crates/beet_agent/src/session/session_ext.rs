//! Helpers for building a session, sessions are expected to
//! have the following hierarchy
//! ```
//! Session
//! 	Actor1
//! 		Message1
//! 			TextContent
//! 			FileContent
//! 		Message2
//! 			TextContent
//! 	Actor2
//! 		Message1
//! 			TextContent
//! 			..
//! ```
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::path::Path;

pub async fn workspace_file(path: impl AsRef<Path>) -> Result<FileContent> {
	session_ext::file(AbsPathBuf::new_workspace_rel(path)?.to_string()).await
}

pub async fn file(path: impl AsRef<str>) -> Result<FileContent> {
	FileContent::new(path).await
}

/// Add a *completed* piece of text content, where no more
/// text will be added to this piece of content.
pub fn text(text: impl AsRef<str>) -> TextContent { TextContent::new(text) }

/// Create a [`Message`] with the given text and file content as children
pub fn message<M>(content: impl IntoContentVec<M>) -> impl Bundle {
	(Message::default(), content.into_content_vec().into_bundle())
}

/// Create a session, inserting the `agent` and `user_message`,
/// then triggering `MessageRequest` on the agent
#[rustfmt::skip]
pub fn user_message_session(agent:impl Bundle, user_message:impl Bundle)->impl Bundle {
	(
		Session::default(),
		children![
			(
				User,
				children![user_message],
			),
			(
				agent,
				OnSpawnBoxed::trigger(MessageRequest)
			)
		]
	)
}


/// Helper to run the provided session, then collects the first
/// emitted file
/// ## Panics
/// Panics if no Agent entity is found
pub async fn run_and_collect_file(session: impl Bundle) -> Vec<ContentEnum> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AgentPlugin));
	app.world_mut().spawn(session);

	AsyncChannel::flush_async_tasks(app.world_mut()).await;

	app.world_mut().run_system_cached(collect_output).unwrap()
}

/// Collect all TextContent and FileContent found under the Agent entity
/// ## Panics
/// Panics if no Agent entity is found
pub fn collect_output(
	agents: Query<Entity, With<Agent>>,
	children: Query<&Children>,
	text: Query<&TextContent>,
	files: Query<&FileContent>,
) -> Vec<ContentEnum> {
	let agent = agents.single().expect("No agent found in world");

	children
		.iter_descendants_depth_first(agent)
		.filter_map(|entity| {
			if let Some(text) = text.get(entity).ok() {
				Some(ContentEnum::Text(text.clone()))
			} else if let Some(file) = files.get(entity).ok() {
				Some(ContentEnum::File(file.clone()))
			} else {
				None
			}
		})
		.collect()
}
