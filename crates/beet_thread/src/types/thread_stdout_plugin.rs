use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::exports::nu_ansi_term::Color;
use beet_core::exports::nu_ansi_term::Style;
use beet_core::prelude::*;

#[derive(Default)]
pub struct ThreadStdoutPlugin;

impl Plugin for ThreadStdoutPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<StdinPost>()
			.add_systems(PreStartup, clear_on_run)
			.add_systems(PostUpdate, (post_added, post_changed).chain())
			.init_resource::<StdoutActorFilter>();
	}
}

fn clear_on_run() -> Result {
	terminal_ext::clear()?;
	println!("");
	Ok(())
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
#[derive(Default, Component)]
struct StdoutCursor {
	pos: u32,
	/// the post is complete and a newline has been printed
	complete: bool,
}

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


		let text = Style::new();
		let primary = Style::new().fg(Color::Cyan);
		let url = Style::new().fg(Color::Blue);
		let error = Style::new().fg(Color::Red).bold();
		let tool_call = Style::new().fg(Color::Yellow);
		let tool_output = Style::new().fg(Color::Yellow);
		let reasoning = Style::new().fg(Color::Green).dimmed();
		primary.paint(actor.name()).xprint();

		// subheading
		match post.as_agent_post() {
			AgentPost::Text(_) => {
				primary.paint(" >").xprint();
			}
			AgentPost::Refusal(_) => {
				error.paint(" - refusal\n").xprint();
			}
			AgentPost::Url(_) => {
				url.paint(" - url\n").xprint();
			}
			AgentPost::Bytes(_) => {
				primary.dimmed().paint(" - bytes\n").xprint();
			}
			AgentPost::Error(_) => {
				error.paint(" - error\n").xprint();
			}
			AgentPost::FunctionCall(_) => {
				tool_call.paint(" - function call\n").xprint();
			}
			AgentPost::FunctionCallOutput(_) => {
				tool_output.paint(" - function call output\n").xprint();
			}
			AgentPost::ReasoningContent(_) | AgentPost::ReasoningSummary(_) => {
				reasoning.paint(" - reasoning..").xprint();
			}
		};

		let mut cursor = StdoutCursor::default();
		// action
		match post.as_agent_post() {
			AgentPost::Bytes(bytes) => {
				text.paint(format!("received bytes: {}", bytes.bytes().len()))
					.xprintln();
			}
			AgentPost::FunctionCall(fc) => {
				text.paint(fc.name()).xprintln();
				print_delta(post, &mut cursor);
			}
			AgentPost::FunctionCallOutput(fc) => {
				text.paint(fc.name().unwrap_or("unknown")).xprintln();
				print_delta(post, &mut cursor);
			}
			AgentPost::ReasoningContent(_) | AgentPost::ReasoningSummary(_) => {
				print_delta(post, &mut cursor);
			}
			_ => {
				print_delta(post, &mut cursor);
			}
		};

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

		if !post.in_progress() {
			// todo print pretty formatted tool call/ output
		}
	}
	std::io::Write::flush(&mut std::io::stdout())?;

	Ok(())
}

fn print_delta(post: &Post, cursor: &mut StdoutCursor) {
	let body = post.to_string();

	let style = match post.as_agent_post() {
		AgentPost::ReasoningContent(_) | AgentPost::ReasoningSummary(_) => {
			Style::default().dimmed()
		}
		_ => Style::default(),
	};

	let new_content = &body[cursor.pos as usize..];
	print!("{}", style.paint(new_content));
	cursor.pos = body.len() as u32;
	if !cursor.complete && !post.in_progress() {
		print!("\n\n");
		cursor.complete = true;
	}
}

#[action]
#[derive(Clone, Component, Reflect)]
#[reflect(Component)]
pub fn StdinPost(
	cx: ActionContext,
	mut query: ThreadQuery,
	actors: Query<&Actor>,
) -> Result<Outcome> {
	let actor = actors.get(cx.id())?;
	let heading = paint_ext::cyan_bold(format!("{} > ", actor.name()));
	print!("{heading}");
	std::io::Write::flush(&mut std::io::stdout())?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	let input = input.trim();

	// Clear the stdin line
	// it will be printed again by the post printer.
	// (up 1 line, erase)
	print!("\x1B[1A\x1B[2K");
	std::io::Write::flush(&mut std::io::stdout())?;

	query.spawn_post(cx.id(), PostStatus::Completed, input)?;
	Ok(Pass(()))
}
