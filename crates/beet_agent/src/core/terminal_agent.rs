use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use std::io::Write;

#[derive(Debug, Clone, Parser)]
pub struct TerminalAgentPlugin {
	/// Initial prompt to start the chat with
	#[arg(
		short = 'p',
		long = "prompt",
		help = "Initial prompt to start the chat"
	)]
	pub initial_prompt: Option<String>,
	/// Trailing positional arguments
	#[arg(
		value_name = "PROMPT",
		trailing_var_arg = true,
		help = "Initial prompt to start the chat"
	)]
	pub trailing_args: Vec<String>,
	/// Paths to files whose contents will be used as the initial prompt
	#[arg(
		short = 'f',
		long = "file",
		value_name = "FILE",
		help = "Path to a file to read the initial prompt from (can be provided multiple times)"
	)]
	pub input_files: Vec<std::path::PathBuf>,
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


	pub fn into_system(&self) -> impl 'static + Fn(Commands) {
		let initial_prompt = if let Some(prompt) = &self.initial_prompt {
			prompt.clone()
		} else if !self.trailing_args.is_empty() {
			self.trailing_args.join(" ")
		} else {
			"Ask me a provocative question".to_string()
		};

		let paths = self
			.input_files
			.iter()
			.map(|path| AbsPathBuf::new_workspace_rel(path).unwrap())
			.collect::<Vec<_>>();

		move |mut commands| {
			let initial_prompt = initial_prompt.clone();
			let paths = paths.clone();
			commands.run_system_cached_with(
				AsyncTask::spawn_with_queue_unwrap,
				async move |queue| {
					let files = async_ext::try_join_all(paths.into_iter().map(
						async |path| {
							FileContent::new(path.to_string_lossy()).await
						},
					))
					.await?;

					queue.with(move |world| {
						let commands = world.commands();
						let mut session = SessionBuilder::new(commands);
						let session_ent = session.session();
						session
							.commands()
							.entity(session_ent)
							.observe(on_content_added)
							.observe(on_content_delta)
							.observe(print_content_ended);
						let mut user = session.add_actor(terminal_user());
						let mut user_msg = user.create_message();
						println!("User > {}\n", initial_prompt);
						user_msg.add_text(initial_prompt);
						for file in files {
							println!("User > {}\n", file);
							user_msg.add_content(file);
						}
						session
							.add_actor(OpenAiProvider::from_env())
							.trigger(StartResponse);
					});
					Ok(())
				},
			);
		}
	}
}
pub enum TerminalAgentMode {
	Oneshot { initial_prompt: String },
}


impl Plugin for TerminalAgentPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AgentPlugin)
			.init_plugin(AsyncPlugin)
			.add_systems(Startup, self.into_system());
	}
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
	if users.contains(ev.actor) {
		// user text already printed
		return;
	}

	let prefix = if users.contains(ev.actor) {
		"User"
	} else if agents.contains(ev.actor) {
		"Agent"
	} else if developers.contains(ev.actor) {
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
	if users.contains(ev.actor) {
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
	if users.contains(ev.actor) {
		// user text already printed
		return;
	}

	print!("\n\n");
}

fn user_input_on_content_end(
	trigger: Trigger<ContentBroadcast<ContentEnded>>,
	commands: Commands,
) -> Result {
	let actor = trigger.target();
	if actor == trigger.actor {
		// println!("ignoring own content");
		return Ok(());
	}
	user_input_request(commands, actor);
	Ok(())
}

fn user_input_request(mut commands: Commands, actor: Entity) {
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue_unwrap,
		async move |queue| {
			use std::io;
			use std::io::Write;

			let stdin = io::stdin();
			let mut input = String::new();
			print!("User > ");
			input.clear();
			let _ = io::stdout().flush();
			let message =
				queue.spawn_then((ChildOf(actor), Message::default())).await;

			match stdin.read_line(&mut input) {
				Ok(0) => {
					// EOF reached
					println!("EOF");
				}
				Ok(_) => {
					// trim trailing newline and print the input
					let line = input.trim_end().to_string();
					let entity = queue
						.spawn_then((ChildOf(message), TextContent::new(line)))
						.await;
					queue.entity(entity).trigger(ContentEnded);
					println!();
				}
				Err(err) => {
					eprintln!("Error reading input: {}", err);
				}
			}
			Ok(())
		},
	);
}
