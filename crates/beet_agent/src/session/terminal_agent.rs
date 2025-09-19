use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use clap::Parser;

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
	#[arg(long = "generate-images", help = "Add the image generation tool")]
	pub generate_images: bool,
	#[clap(flatten)]
	pub config: TerminalAgentConfig,
}


macro_rules! print_flush {
 ($($arg:tt)*) => {{
  use std::io::{self, Write};
  print!($($arg)*);
  let _ = io::stdout().flush();
 }};
}


#[derive(Debug, Clone, Parser, Resource)]
pub struct TerminalAgentConfig {
	#[arg(
		long,
		help = "Run in oneshot mode, exiting after the first message received"
	)]
	oneshot: bool,
	#[arg(short, long, help = "Directory to write received files to")]
	out_dir: Option<std::path::PathBuf>,
}

impl Plugin for TerminalAgentPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AgentPlugin)
			.init_plugin(AsyncPlugin)
			.insert_resource(self.config.clone())
			.add_systems(Startup, self.into_system())
			.add_observer(text_added)
			.add_observer(text_delta)
			.add_observer(reasoning_added)
			.add_observer(reasoning_ended)
			.add_observer(message_ended)
			.add_observer(file_inserted)
			.add_observer(route_message_requests);
	}
}

impl TerminalAgentPlugin {
	pub fn into_system(&self) -> impl 'static + Fn(Commands) {
		let initial_prompt = if let Some(prompt) = &self.initial_prompt {
			Some(prompt.clone())
		} else if !self.trailing_args.is_empty() {
			Some(self.trailing_args.join(" "))
		} else {
			None
		};

		let paths = self
			.input_files
			.iter()
			.map(|path| AbsPathBuf::new_workspace_rel(path).unwrap())
			.collect::<Vec<_>>();
		let generate_images = self.generate_images;

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

						let mut user = session.add_actor(terminal_user());
						let mut user_msg = user.create_message();
						if let Some(initial_prompt) = &initial_prompt {
							println!("User > {}\n", initial_prompt);
							user_msg.add_text(initial_prompt);
						}
						let users_turn =
							initial_prompt.is_none() && files.is_empty();
						for file in files {
							println!("User > {}\n", file);
							user_msg.add_content(file);
						}
						if users_turn {
							user.trigger(MessageRequest);
						}
						let mut provider = OpenAiAgent::from_env();
						if generate_images {
							provider =
								provider.with_tool(GenerateImage::default());
						}
						let mut agent = session.add_actor(provider);
						if !users_turn {
							agent.trigger(MessageRequest);
						}
					});
					Ok(())
				},
			);
		}
	}
}



fn terminal_user() -> impl Bundle {
	(User, EntityObserver::new(user_message_request))
}

fn text_added(ev: Trigger<OnAdd, TextContent>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.target())?;
	if actor.role != ActorRole::User {
		print_flush!("{} > ", actor.role);
	}
	Ok(())
}
fn text_delta(ev: Trigger<TextDelta>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.target())?;
	if actor.role != ActorRole::User {
		print_flush!("{}", ev.event().0);
	}
	Ok(())
}

fn reasoning_added(
	ev: Trigger<OnAdd, ReasoningContent>,
	cx: SessionParams,
) -> Result {
	let actor = cx.actor(ev.target())?;
	print_flush!("{} > reasoning...", actor.role);

	Ok(())
}

fn reasoning_ended(
	ev: Trigger<OnAdd, ContentEnded>,
	query: Query<(), With<ReasoningContent>>,
) -> Result {
	if query.contains(ev.target()) {
		print_flush!(" done\n\n");
	}
	Ok(())
}

fn file_inserted(
	ev: Trigger<OnInsert, FileContent>,
	mut cache: Local<HashMap<String, AbsPathBuf>>,
	cx: SessionParams,
	config: Res<TerminalAgentConfig>,
	query: Query<&FileContent>,
	mut commands: Commands,
) -> Result {
	let file = query.get(ev.target())?;
	let actor = cx.actor(ev.target())?;
	if actor.role != ActorRole::User {
		print_flush!("{} > file: {}", actor.role, file);
		let cache_len = cache.len();
		let filename = cache
			.entry(file.filename.to_string_lossy().to_string())
			.or_insert_with(|| {
				let base = config.out_dir.clone().unwrap_or_default();
				let base = base.display();
				AbsPathBuf::new_workspace_rel(format!(
					"{base}/file{cache_len}.{}",
					file.extension()
				))
				.unwrap()
			})
			.clone();
		let file = file.clone();
		commands.run_system_cached_with(
			AsyncTask::spawn_with_queue_unwrap,
			async move |_| {
				let data = file.data.get().await?;
				FsExt::write_async(filename, data).await?;
				Ok(())
			},
		);
	}
	Ok(())
}

fn message_ended(
	ev: Trigger<OnAdd, MessageComplete>,
	cx: SessionParams,
) -> Result {
	let actor = cx.actor(ev.target())?;
	if actor.role != ActorRole::User {
		print_flush!("\n\n");
	}
	Ok(())
}

fn route_message_requests(
	ev: Trigger<OnAdd, MessageComplete>,
	cx: SessionParams,
	config: Res<TerminalAgentConfig>,
	mut commands: Commands,
	users: Query<Entity, With<User>>,
	agents: Query<Entity, With<Agent>>,
) -> Result {
	let actor = cx.actor(ev.target())?;
	match actor.role {
		ActorRole::User => {
			commands.entity(agents.single()?).trigger(MessageRequest);
		}
		ActorRole::Agent if config.oneshot => {
			commands.send_event(AppExit::Success);
		}
		ActorRole::Agent => {
			commands.entity(users.single()?).trigger(MessageRequest);
		}
		_ => {}
	}

	Ok(())
}

fn user_message_request(
	ev: Trigger<MessageRequest>,
	mut commands: Commands,
	cx: SessionParams,
) -> Result {
	let actor = cx.actor(ev.target())?.entity;
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue_unwrap,
		async move |queue| {
			use std::io;
			use std::io::Write;

			let stdin = io::stdin();
			let mut input = String::new();
			print_flush!("User > ");
			input.clear();
			let _ = io::stdout().flush();

			let mut spawner =
				MessageSpawner::spawn(queue.clone(), actor).await?;
			match stdin.read_line(&mut input) {
				Ok(0) => {
					// EOF reached
					println!("EOF");
				}
				Ok(_) => {
					// trim trailing newline and print the input
					let line = input.trim_end().to_string();
					let id = 0;
					spawner
						.add(
							id,
							(TextContent::new(line), ContentEnded::default()),
						)
						.await?
						.finish_message()
						.await?;
					println!();
				}
				Err(err) => {
					eprintln!("Error reading input: {}", err);
				}
			}
			Ok(())
		},
	);
	Ok(())
}
