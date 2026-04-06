use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

#[derive(Default)]
pub struct ThreadStdoutPlugin;

impl Plugin for ThreadStdoutPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<StdinPost>()
			.add_systems(PostUpdate, (post_added, post_changed).chain())
			.init_resource::<StdoutActorFilter>();
	}
}

/// Filter to determine which actor messages
/// are printed to stdout, defaults to all
#[derive(Deref, Resource)]
pub struct StdoutActorFilter(Vec<ActorKind>);


impl Default for StdoutActorFilter {
	fn default() -> Self {
		Self(vec![
			ActorKind::System,
			ActorKind::Developer,
			ActorKind::Agent,
			ActorKind::User,
		])
	}
}

// cursor to track which part of post deltas have already been printed
#[derive(Default, Deref, DerefMut, Component)]
struct StdoutCursor(u32);

fn post_added(
	filter: Res<StdoutActorFilter>,
	mut commands: Commands,
	query: Populated<(Entity, &Post), Added<Post>>,
	thread_query: ThreadQuery,
) -> Result {
	// handle multiple simultaneously created posts,
	// ie scene load
	let mut posts: Vec<_> = query.iter().collect();
	posts.sort_by_key(|(_, post)| *post);

	for (entity, post) in posts {
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
			"refusal > "
		} else if agent_post.is_reasoning_summary()
			|| agent_post.is_reasoning_content()
		{
			"thinking.. "
		} else if agent_post.is_url() || agent_post.is_bytes() {
			"media "
		} else {
			"> "
		};

		let heading =
			paint_ext::cyan_bold(format!("\n\n{} {}", actor.name(), suffix));
		print!("{heading}");

		let mut cursor = StdoutCursor::default();
		print_delta(post, &mut cursor);
		commands.entity(entity).insert(cursor);
	}
	std::io::Write::flush(&mut std::io::stdout())?;

	Ok(())
}


fn post_changed(
	filter: Res<StdoutActorFilter>,
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
		print_delta(post, &mut cursor);
	}
	std::io::Write::flush(&mut std::io::stdout())?;

	Ok(())
}

fn print_delta(post: &Post, cursor: &mut StdoutCursor) {
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

#[tool]
#[derive(Clone, Component, Reflect)]
#[reflect(Component)]
pub fn StdinPost(
	cx: SystemToolIn,
	mut query: ThreadQuery,
	actors: Query<&Actor>,
) -> Result<Outcome> {
	let actor = actors.get(cx.caller)?;
	let heading = paint_ext::cyan_bold(format!("{} > ", actor.name()));
	// reserve extra line to prevent jump
	print!("\n\n\n\x1B[1A{heading}");
	std::io::Write::flush(&mut std::io::stdout())?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	let input = input.trim();

	// Clear the terminal line
	// (up 1 line, erase, up 2 lines)
	// it will be printed again by the printer.
	print!("\x1B[1A\x1B[2K\x1B[1A\x1B[1A");
	std::io::Write::flush(&mut std::io::stdout())?;

	query.spawn_post(cx.caller, PostStatus::Completed, input)?;
	Ok(Pass(()))
}
