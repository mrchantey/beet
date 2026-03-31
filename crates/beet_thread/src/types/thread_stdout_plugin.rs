use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct ThreadStdoutPlugin;

impl Plugin for ThreadStdoutPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, (post_added, post_changed).chain())
			.init_resource::<ActorFilter>();
	}
}


#[derive(Deref, Resource)]
pub struct ActorFilter(Vec<ActorKind>);


impl Default for ActorFilter {
	fn default() -> Self {
		Self(vec![
			ActorKind::System,
			ActorKind::Developer,
			ActorKind::Agent,
		])
	}
}

// cursor to track which part of post deltas have already been printed
#[derive(Default, Deref, DerefMut, Component)]
struct StdoutCursor(u32);

fn post_added(
	filter: Res<ActorFilter>,
	mut commands: Commands,
	query: Populated<(Entity, &Post), Added<Post>>,
	thread_query: ThreadQuery,
) -> Result {
	for (entity, post) in query.iter() {
		commands.entity(entity).insert(StdoutCursor::default());
		let actor = thread_query.actor_from_post_entity(entity)?;

		if !filter.contains(&actor.kind()) {
			continue;
		}

		// hide reasoning in release builds
		#[cfg(not(debug_assertions))]
		if !post.intent().is_display() {
			continue;
		}

		let agent_post = post.as_agent_post();
		let suffix = if agent_post.is_refusal() {
			"refusal >"
		} else if agent_post.is_reasoning_summary()
			|| agent_post.is_reasoning_content()
		{
			"thinking.. "
		} else if agent_post.is_url() || agent_post.is_bytes() {
			"media "
		} else {
			">"
		};

		let heading =
			paint_ext::cyan_bold(format!("\n{} {}\n", actor.name(), suffix));
		println!("{heading}");
	}

	Ok(())
}

fn post_changed(
	filter: Res<ActorFilter>,
	mut query: Populated<(Entity, &Post, &mut StdoutCursor), Changed<Post>>,
	thread_query: ThreadQuery,
) -> Result {
	for (entity, post, mut cursor) in query.iter_mut() {
		let actor = thread_query.actor_from_post_entity(entity)?;
		if !filter.contains(&actor.kind()) {
			continue;
		}

		// hide reasoning and tool calls in release builds
		#[cfg(not(debug_assertions))]
		if !post.intent().is_display() {
			continue;
		}
		let body = post.to_string();

		let new_content = &body[**cursor as usize..];
		let agent_post = post.as_agent_post();
		let colored = if agent_post.is_refusal() {
			paint_ext::red(new_content)
		} else if agent_post.is_reasoning_summary()
			|| agent_post.is_reasoning_content()
		{
			paint_ext::dimmed(new_content)
		} else {
			new_content.to_string()
		};

		print!("{}", colored);
		**cursor = body.len() as u32;
	}

	Ok(())
}
