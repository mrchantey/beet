use beet_mcp::prelude::*;
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::INFO);
	let mcp_client = McpClient::new_stdio_dev().await?;
	// model for how the agent discovers and uses tools, this can be different from
	// the rag model
	let tools_embedding_model = EmbedModel::all_minilm();

	// let agent = CompletionModel::deepseek() // Error: tools not supported?
	let agent = ChatModel::gpt_4o()
		.preamble("Talk like an old sea farer.")
		.add_mcp_tools(&mcp_client, tools_embedding_model)
		.await?
		.temperature(0.5)
		.build();


	let response = agent
		.prompt("lets create a simple 3d scene in the crate bevy 0.16.0")
		// .prompt("ahoy matey, how does the resonance work in nexus arcana?")
		.await?;

	println!("Response: {}", response);

	// fuzzy test that it actually read the tool
	assert!(response.contains(".add_systems(Startup, setup)"));
	let response = agent
		.prompt("lets create a simple 3d scene in the crate bevy 0.4.0")
		.await?;

	println!("Response: {}", response);

	// fuzzy test that it actually read the tool
	assert!(response.contains(".add_startup_system(setup.system())"));
	Ok(())
}
