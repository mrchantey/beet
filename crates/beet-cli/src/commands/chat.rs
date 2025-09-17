use beet::prelude::*;
use clap::Parser;

/// Start a chat application
#[derive(Debug, Clone, Parser)]
pub struct Chat {}



impl Chat {
	pub async fn run(self) -> Result {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TerminalChatPlugin));
		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
	}
}
