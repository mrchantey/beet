use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::io::Write;



pub struct TerminalChatPlugin;

impl Plugin for TerminalChatPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AgentPlugin)
			.init_plugin(AsyncPlugin)
			.add_systems(Startup, setup);
	}
}

fn setup(mut commands: Commands) {
	let mut session = SessionBuilder::new(commands.reborrow());
	let session_ent = session.session();
	session
		.commands()
		.entity(session_ent)
		.observe(on_content_added)
		.observe(on_content_delta)
		.observe(print_content_ended);
	let user = session.add_member(terminal_user());
	let _agent = session.add_member(open_ai_provider());
	let initial_prompt = "tell me story";
	session.add_content(user, initial_prompt);
	println!("User > {}", initial_prompt);
}


fn terminal_user() -> impl Bundle {
	(User, EntityObserver::new(user_input_on_content_end))
}

fn on_content_added(
	trigger: Trigger<ContentBroadcast<ContentAdded>>,
	users: Query<Entity, With<User>>,
	agents: Query<Entity, With<Agent>>,
	developers: Query<Entity, With<Developer>>,
) {
	if users.contains(trigger.owner) {
		// user text already printed
		return;
	}

	let prefix = if users.contains(trigger.owner) {
		"User"
	} else if agents.contains(trigger.owner) {
		"Agent"
	} else if developers.contains(trigger.owner) {
		"Developer"
	} else {
		"Unknown"
	};
	print!("{prefix} > ");
}
fn on_content_delta(
	trigger: Trigger<ContentBroadcast<ContentTextDelta>>,
	users: Query<Entity, With<User>>,
) {
	if users.contains(trigger.owner) {
		// user text already printed
		return;
	}
	let text = trigger.event().event.clone().0;
	print!("{}", text);
	let _ = std::io::stdout().flush();
}
fn print_content_ended(_: Trigger<ContentBroadcast<ContentEnded>>) {
	print!("\n");
}

fn user_input_on_content_end(
	trigger: Trigger<ContentBroadcast<ContentEnded>>,
	commands: Commands,
) -> Result {
	let ContentBroadcast {
		session,
		owner: content_owner,
		..
	} = trigger.event().clone();

	let member_ent = trigger.target();
	if member_ent == content_owner {
		// println!("ignoring own content");
		return Ok(());
	}
	user_input_request(commands, session, member_ent);
	Ok(())
}

fn user_input_request(
	mut commands: Commands,
	session: Entity,
	user_member: Entity,
) {
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue,
		async move |queue| {
			use std::io;
			use std::io::Write;

			let stdin = io::stdin();
			let mut input = String::new();
			print!("User > ");
			input.clear();
			let _ = io::stdout().flush();
			match stdin.read_line(&mut input) {
				Ok(0) => {
					// EOF reached
					println!("EOF");
				}
				Ok(_) => {
					// trim trailing newline and print the input
					let line = input.trim_end().to_string();
					let entity = queue
						.spawn_then(text_content(session, user_member, line))
						.await;
					queue.entity(entity).trigger(ContentEnded);
				}
				Err(err) => {
					eprintln!("Error reading input: {}", err);
				}
			}
			// Ok(())
		},
	);
}
