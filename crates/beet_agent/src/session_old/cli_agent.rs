use crate::prelude::Message;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::spawn::SpawnIter;

#[derive(Debug, Clone)]
pub struct CliAgentPlugin {
	/// Initial prompt to start the chat
	pub initial_prompt: Option<String>,
	/// Initial prompt to start the chat
	pub initial_prompt_trailing: Vec<String>,
	/// Paths to files to be included in the initial prompt
	pub input_files: Vec<std::path::PathBuf>,
	/// Add the image generation tool
	pub generate_images: bool,
	pub config: CliAgentConfig,
}

impl Default for CliAgentPlugin {
	fn default() -> Self {
		Self {
			initial_prompt: None,
			initial_prompt_trailing: Vec::new(),
			input_files: Vec::new(),
			generate_images: false,
			config: CliAgentConfig::default(),
		}
	}
}

impl CliAgentPlugin {
	pub fn initial_message(&self) -> bool {
		self.initial_prompt.is_some()
			|| !self.initial_prompt_trailing.is_empty()
			|| !self.input_files.is_empty()
	}
}

/// Print the text then immediately flush stdout
macro_rules! print_flush {
 ($($arg:tt)*) => {{
  use std::io::{self, Write};
  print!($($arg)*);
  let _ = io::stdout().flush();
 }};
}


#[derive(Debug, Clone, Resource)]
pub struct CliAgentConfig {
	/// Run in oneshot mode, exiting after the first message received
	oneshot: bool,
	/// Path without extension to write output images and text to
	out_file: Option<std::path::PathBuf>,
	/// Overwrite existing files instead of creating new ones
	overwrite: bool,
}

impl Default for CliAgentConfig {
	fn default() -> Self {
		Self {
			oneshot: false,
			out_file: None,
			overwrite: false,
		}
	}
}

impl CliAgentConfig {
	pub fn oneshot(&self) -> bool { self.oneshot }

	pub fn next_available_filename(
		&self,
		extension: &str,
	) -> Result<AbsPathBuf> {
		let mut filename =
			self.out_file.clone().unwrap_or_else(|| "file".into());
		filename.set_extension(extension);
		let file_stem = filename
			.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or("file")
			.to_string();
		let mut path = AbsPathBuf::new_workspace_rel(filename)?;
		if self.overwrite {
			path.xok()
		} else {
			let mut suffix = 0;
			while path.exists() {
				suffix += 1;
				path.set_file_name(format!(
					"{}-{}.{}",
					file_stem, suffix, extension
				));
			}
			path.xok()
		}
	}
}

impl Plugin for CliAgentPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AgentPlugin>()
			.init_plugin::<AsyncPlugin>()
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

impl CliAgentPlugin {
	pub fn into_system(&self) -> impl 'static + Fn(AsyncCommands) {
		let initial_prompt = if let Some(prompt) = &self.initial_prompt {
			Some(prompt.clone())
		} else if !self.initial_prompt_trailing.is_empty() {
			Some(self.initial_prompt_trailing.join(" "))
		} else {
			None
		};

		let initial_message = self.initial_message();

		let paths = self
			.input_files
			.iter()
			.map(|path| AbsPathBuf::new_workspace_rel(path).unwrap())
			.collect::<Vec<_>>();
		let generate_images = self.generate_images;

		move |mut commands| {
			let initial_prompt = initial_prompt.clone();
			let paths = paths.clone();
			commands.run_local(async move |queue| {
				let files =
					async_ext::try_join_all(paths.into_iter().map(|path| {
						FileContent::new(path.to_string_lossy().to_string())
					}))
					.await?;

				queue.with(move |world| {
					#[rustfmt::skip]
						world.spawn((
							Session::default(),
							children![
								// Actor 1: User
								(
									terminal_user(),
									OnSpawn::trigger_option(initial_message.xmap_false(||{
										MessageRequest
									})),
									children![
										// Initial User Message
										(
											Message::default(),
											children![
												OnSpawn::insert_option(
													initial_prompt.map(|prompt| {
														println!("User > {}\n", prompt);
														session_ext::text(prompt)
													})),
												Children::spawn(SpawnIter(files.into_iter().map(|file|{
													println!("User > {}\n", file);
													(file, ContentEnded::default())
												})))
											]
										)]
								),
							// Actor 2: Agent
							(
								{
									let mut provider = GeminiAgent::from_env();
									if generate_images {
										provider =
											provider.with_model(GEMINI_2_5_FLASH_IMAGE);
									}
									provider
								},
								OnSpawn::trigger_option(initial_message.xmap_true(||{
									MessageRequest
								})),
							)
						]));
				});
				Ok(())
			});
		}
	}
}



fn terminal_user() -> impl Bundle {
	(UserRole, OnSpawn::observe(user_message_request))
}

fn text_added(ev: On<Add, TextContent>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.event_target())?;
	if actor.role != ActorRole::User {
		print_flush!("\n{} > ", actor.role);
	}
	Ok(())
}
fn text_delta(ev: On<TextDelta>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.event_target())?;
	if actor.role != ActorRole::User {
		print_flush!("{}", ev.value);
	}
	Ok(())
}

fn reasoning_added(ev: On<Add, ReasoningContent>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.event().event_target())?;
	print_flush!("{} > 🤔", actor.role);

	Ok(())
}

fn reasoning_ended(
	ev: On<Add, ContentEnded>,
	query: Query<(), With<ReasoningContent>>,
) -> Result {
	if query.contains(ev.event().event_target()) {
		print_flush!(" 💡\n");
	}
	Ok(())
}

fn file_inserted(
	ev: On<Insert, FileContent>,
	cx: SessionParams,
	config: Res<CliAgentConfig>,
	query: Query<&FileContent>,
	mut commands: AsyncCommands,
) -> Result {
	let file = query.get(ev.event().event_target())?;
	let actor = cx.actor(ev.event().event_target())?;
	if actor.role != ActorRole::User {
		let filename = config.next_available_filename(file.extension())?;
		print_flush!("\n{} > file: {}", actor.role, filename);
		let file = file.clone();
		commands.run_local(async move |_| {
			let data = file.data.get().await?;
			fs_ext::write_async(filename, data).await?;
			Ok(())
		});
	}
	Ok(())
}

fn message_ended(ev: On<Add, MessageComplete>, cx: SessionParams) -> Result {
	let actor = cx.actor(ev.event().event_target())?;
	if actor.role != ActorRole::User {
		print_flush!("\n");
	}
	Ok(())
}

fn route_message_requests(
	ev: On<Add, MessageComplete>,
	cx: SessionParams,
	config: Res<CliAgentConfig>,
	mut commands: Commands,
	users: Query<Entity, With<UserRole>>,
	agents: Query<Entity, With<AgentRole>>,
) -> Result {
	let actor = cx.actor(ev.event().event_target())?;
	match actor.role {
		ActorRole::User => {
			commands.entity(agents.single()?).trigger(MessageRequest);
		}
		ActorRole::Agent if config.oneshot => {
			commands.write_message(AppExit::Success);
		}
		ActorRole::Agent => {
			commands.entity(users.single()?).trigger(MessageRequest);
		}
		_ => {}
	}

	Ok(())
}

fn user_message_request(
	ev: On<MessageRequest>,
	mut commands: AsyncCommands,
	cx: SessionParams,
) -> Result {
	let actor = cx.actor(ev.event_target())?.entity;
	commands.run_local(async move |queue| {
		use std::io;
		use std::io::Write;

		let stdin = io::stdin();
		let mut input = String::new();
		print_flush!("\nUser > ");
		input.clear();
		let _ = io::stdout().flush();

		let mut spawner = MessageSpawner::spawn(queue.clone(), actor).await?;
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
					.add(id, (TextContent::new(line), ContentEnded::default()))
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
	});
	Ok(())
}
