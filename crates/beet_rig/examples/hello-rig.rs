use rig::completion::Prompt;
use rig::providers;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let client = providers::openai::Client::from_env();

	let agent = client
        .agent("gpt-4o")
        .preamble("You are a comedian here to entertain the user using humour and jokes.")
        .build();

	let response = agent.prompt("Entertain me!").await?;
	println!("{}", response);

	Ok(())
}
