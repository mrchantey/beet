use beet_mcp::prelude::*;
use rig::cli_chatbot::cli_chatbot;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::DEBUG);
	// Print the current tracing level
	let mcp_client = McpClient::new_stdio_dev().await?;

	let agent = AgentModel::from_env()
		.preamble("Talk like an old sea farer.")
		.add_mcp_tools(&mcp_client, EmbedModel::from_env())
		.await?
		.temperature(0.5)
		.build();

	cli_chatbot(agent).await?;

	Ok(())
}
