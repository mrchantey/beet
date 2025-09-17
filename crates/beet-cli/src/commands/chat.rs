use beet::prelude::*;
use clap::Parser;

/// Start a chat application
#[derive(Debug, Clone, Parser)]
pub struct Chat {
	/// Initial prompt to start the chat with
	#[arg(
		short = 'p',
		long = "prompt",
		help = "Initial prompt to start the chat"
	)]
	pub initial_prompt: Option<String>,
	/// Trailing positional arguments passed after `--`
	#[arg(
		value_name = "ARGS",
		trailing_var_arg = true,
		help = "Trailing arguments"
	)]
	pub trailing_args: Vec<String>,
}



impl Chat {
	pub async fn run(self) -> Result {
		let mut app = App::new();

		let initial_prompt = if let Some(prompt) = self.initial_prompt {
			prompt
		} else if !self.trailing_args.is_empty() {
			self.trailing_args.join(" ")
		} else {
			"ask me a provocative question".to_string()
		};


		app.add_plugins((MinimalPlugins, TerminalChatPlugin {
			initial_prompt,
		}));
		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
	}
}
