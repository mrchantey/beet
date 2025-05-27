use beet_mcp::prelude::*;
use rig::cli_chatbot::cli_chatbot;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::ERROR);
	let mcp_client = McpClient::new_stdio_dev().await?;
	// model for how the agent discovers and uses tools, this can be different from
	// the rag model
	let tools_embedding_model = EmbedModel::all_minilm();

	// let agent = CompletionModel::deepseek() // Error: tools not supported?
	let agent = ChatModel::gpt_4o()
		.preamble(
			"Be clear and concise, using the provided mcp tools to answer questions",
		)
		.add_mcp_tools(&mcp_client, tools_embedding_model)
		.await?
		.temperature(0.5)
		.build();

	cli_chatbot(agent).await?;

	Ok(())
}
