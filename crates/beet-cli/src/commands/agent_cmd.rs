use beet::prelude::*;
use clap::Parser;

/// Start a chat application
#[derive(Debug, Clone, Parser)]
pub struct AgentCmd {
	#[clap(flatten)]
	plugin: CliAgentPlugin,
}



impl AgentCmd {
	pub async fn run(self) -> Result {
		App::new()
			.add_plugins((MinimalPlugins, self.plugin))
			.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
	}
}
