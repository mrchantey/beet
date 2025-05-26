use beet_mcp::prelude::*;
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing();
	let mcp_client = McpClient::new_stdio_dev().await?;
	let tools_embedding_model = EmbedModel::all_minilm();

	// let agent = CompletionModel::deepseek() // Error: tools not supported?
	let agent = ChatModel::gpt_4o()
		.preamble("Talk like an old sea farer.")
		.add_mcp_tools(&mcp_client, tools_embedding_model)
		.await?
		.temperature(0.5)
		.build();


	// note that the agent knows to exclude the greeting 'ahoy matey' from the 'Tool Call'
	let response = agent
		.prompt("ahoy matey, how does the resonance work in nexus arcana?")
		.await?;

	println!("Response: {}", response);

	// fuzzy test that it actuayll read the tool
	assert!(response.contains("magic"));
	assert!(response.contains("technology"));
	Ok(())
}
