use beet_mcp::prelude::*;
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing();
	let mcp_client = McpClient::new_stdio_dev().await?;
	let tools_embedding_model = EmbedModel::all_minilm();

	let agent = CompletionModel::gpt_4o()
		.preamble("Talk like an old sea farer.")
		.add_mcp_tools(&mcp_client, tools_embedding_model)
		.await?
		.temperature(0.5)
		.build();

	let response = agent
		.prompt("how does the resonance work in nexus arcana?")
		.await?;
	println!("Response: {}", response);

	Ok(())
}
