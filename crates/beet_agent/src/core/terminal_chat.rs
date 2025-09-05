use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;



pub struct TerminalChatPlugin;

impl Plugin for TerminalChatPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AgentPlugin)
			.add_systems(Startup, user_input);
	}
}

fn user_input(mut commands: Commands) {
	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue,
		async |mut _queue| {
			use std::io;
			use std::io::Write;

			let stdin = io::stdin();
			let mut input = String::new();

			loop {
				input.clear();
				// prompt
				print!("> ");
				let _ = io::stdout().flush();

				match stdin.read_line(&mut input) {
					Ok(0) => {
						// EOF reached
						println!("EOF");
						break;
					}
					Ok(_) => {
						// trim trailing newline and print the input
						let line = input.trim_end().to_string();
						println!("{}", line);
					}
					Err(err) => {
						eprintln!("Error reading input: {}", err);
						break;
					}
				}
			}
			3
		},
	);
	// commands.run_system_cached_with(
	// 	AsyncTask::spawn_with_queue_then,
	// 	async move |_| {
	// 	},
	// );
}
