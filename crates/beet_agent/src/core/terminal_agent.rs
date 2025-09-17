use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::io::Write;



pub struct TerminalAgentPlugin {
	pub initial_prompt: String,
}
impl TerminalAgentPlugin {
	/// Exit after the first agent response
	pub fn oneshot() -> impl Bundle {
		Observer::new(
			|ev: Trigger<ContentBroadcast<ContentEnded>>,
			 agents: Query<(), With<Agent>>,
			 mut exit: EventWriter<AppExit>| {
				if agents.contains(ev.target()) {
					exit.write(AppExit::Success);
				}
			},
		)
	}
}
pub enum TerminalAgentMode {
	Oneshot { initial_prompt: String },
}


impl Plugin for TerminalAgentPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AgentPlugin).init_plugin(AsyncPlugin);
		app.world_mut()
			.run_system_cached_with(setup, self.initial_prompt.clone())
			.unwrap();
	}
}

fn setup(initial_prompt: In<String>, mut commands: Commands) {
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
	println!("User > {}\n", initial_prompt.0);
	session.add_content(user, initial_prompt.0);
}


fn terminal_user() -> impl Bundle {
	(User, EntityObserver::new(user_input_on_content_end))
}

fn on_content_added(
	ev: Trigger<ContentBroadcast<ContentAdded>>,
	users: Query<(), With<User>>,
	agents: Query<(), With<Agent>>,
	developers: Query<(), With<Developer>>,
) {
	if users.contains(ev.owner) {
		// user text already printed
		return;
	}

	let prefix = if users.contains(ev.owner) {
		"User"
	} else if agents.contains(ev.owner) {
		"Agent"
	} else if developers.contains(ev.owner) {
		"Developer"
	} else {
		"Unknown"
	};
	print!("{prefix} > ");
}
fn on_content_delta(
	ev: Trigger<ContentBroadcast<ContentTextDelta>>,
	users: Query<(), With<User>>,
) {
	if users.contains(ev.owner) {
		// user text already printed
		return;
	}
	let text = ev.event().event.clone().0;
	print!("{}", text);
	let _ = std::io::stdout().flush();
}
fn print_content_ended(
	ev: Trigger<ContentBroadcast<ContentEnded>>,
	users: Query<(), With<User>>,
) {
	if users.contains(ev.owner) {
		// user text already printed
		return;
	}

	print!("\n\n");
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
					println!();
				}
				Err(err) => {
					eprintln!("Error reading input: {}", err);
				}
			}
			// Ok(())
		},
	);
}
