//! Demonstrates tool calling with the [`ThreadMut`] fluent API.
//!
//! The agent is asked about the weather and calls the `get_weather`
//! tool to produce structured output.
use beet::prelude::*;


#[derive(Reflect)]
pub struct DiscoverThingsInput {
	question: String,
}

#[beet::main]
async fn main() {
	let posts = ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post("Tell me about flimflams?")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(OllamaProvider::qwen().without_streaming())
		.with_child(function_tool(
			"discover-things",
			"learn all about things like flim-flams",
			func_tool(|cx: FuncToolIn<DiscoverThingsInput>| {
				let question = cx.question.clone();

				format!(
					"you asked {}, great question!
				flimflams are used by gzorps
					",
					question
				)
				.xok()
			}),
		))
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
