use beet_mcp::prelude::*;
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::DEBUG);
	let mcp_client = McpClient::new_stdio_dev().await?;

	let agent = AgentModel::from_env()
		.preamble("Talk like an old sea farer.")
		.add_mcp_tools(&mcp_client, EmbedModel::from_env())
		.await?
		.temperature(0.5)
		.build();


	let response = agent
		.prompt("code snippet for a fps camera controller in bevy 0.16")
		.await?;

	println!("✨ Response:\n{}", response);

	Ok(())
}
