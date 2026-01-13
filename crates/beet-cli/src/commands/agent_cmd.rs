use beet::prelude::*;

/// Start a chat application
#[derive(Debug, Clone, Default)]
pub struct AgentCmd {
	plugin: CliAgentPlugin,
}



impl AgentCmd {
	pub async fn run(self) -> Result {
		App::new()
			.add_plugins((MinimalPlugins, self.plugin))
			.run_async()
			.await
			.into_result()
	}
}
