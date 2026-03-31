//! Demonstrates tool calling with the [`ThreadMut`] fluent API.
//!
//! The agent is asked about the weather and calls the `get_weather`
//! tool to produce structured output.
use beet::prelude::*;

#[beet::main]
async fn main() {
	let tool = FunctionTool::new(
		"get_weather",
		"Get the current weather for a location",
		serde_json::json!({
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "The city and state, e.g. San Francisco, CA"
				}
			},
			"required": ["location"]
		}),
	);

	let posts = ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post("What's the weather like in San Francisco?")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(OllamaProvider::qwen().without_streaming())
		.with_tool(tool)
		.send_and_collect()
		.await
		.unwrap();

	for post in &posts {
		match post.as_agent_post() {
			AgentPost::FunctionCall(fc) => {
				println!("Tool call: {}({})", fc.name(), fc.arguments());
			}
			_ if post.intent().is_display() => {
				println!("Response: {post}");
			}
			_ => {}
		}
	}
}
