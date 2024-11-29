use anyhow::Result;
use rig::completion::Prompt;
use rig::completion::ToolDefinition;
use rig::providers;
use rig::tool::Tool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

#[derive(Deserialize)]
struct OperationArgs {
	x: i32,
	y: i32,
	is_pizza: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;
impl Tool for Adder {
	const NAME: &'static str = "add";

	type Error = MathError;
	type Args = OperationArgs;
	type Output = i32;

	async fn definition(&self, _prompt: String) -> ToolDefinition {
		ToolDefinition {
			name: Self::NAME.to_string(),
			description: "Add x and y together, additionally say whether the user is a pizza".to_string(),
			parameters: json!({
				"type": "object",
				"properties": {
					"x": {
						"type": "number",
						"description": "The first number to add"
					},
					"y": {
						"type": "number",
						"description": "The second number to add"
					},
                    "is_pizza": {
						"type": "string",
						"description": "Whether the user is a pizza"
					}
				}
			}),
		}
	}

	async fn call(
		&self,
		args: Self::Args,
	) -> Result<Self::Output, Self::Error> {
		let result = args.x + args.y;
		Ok(result)
	}
}

#[derive(Deserialize, Serialize)]
struct Subtract;
impl Tool for Subtract {
	const NAME: &'static str = "subtract";

	type Error = MathError;
	type Args = OperationArgs;
	type Output = i32;

	async fn definition(&self, _prompt: String) -> ToolDefinition {
		serde_json::from_value(json!({
			"name": "subtract",
			"description": "Subtract y from x (i.e.: x - y),, additionally say whether the user is a pizza",
			"parameters": {
				"type": "object",
				"properties": {
					"x": {
						"type": "number",
						"description": "The number to substract from"
					},
					"y": {
						"type": "number",
						"description": "The number to substract"
					},
					"is_pizza": {
						"type": "string",
						"description": "Whether the user is a pizza"
					}
				}
			}
		}))
		.expect("Tool Definition")
	}

	async fn call(
		&self,
		args: Self::Args,
	) -> Result<Self::Output, Self::Error> {
        println!("{}",args.is_pizza);
		let result = args.x - args.y;
		Ok(result)
	}
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Create OpenAI client
	let openai_client = providers::openai::Client::from_env();

	// Create agent with a single context prompt and two tools
	let calculator_agent = openai_client
        .agent(providers::openai::GPT_4O)
        .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question. Also it is very important when using tools to specify whether the user is a pizza or not.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)
        .build();

	// Prompt the agent and print the response
	println!("Calculate 2 - 5");
	println!(
		"Calculator Agent: {}",
		calculator_agent.prompt("Calculate 2 - 5").await?
	);

	Ok(())
}
