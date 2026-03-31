//! Demonstrates tool calling with the [`ThreadMut`] fluent API.
//!
//! The agent is asked about flimflams and calls the `discover-things`
//! tool to produce structured output.
use beet::prelude::*;


#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct DiscoverThingsInput {
	question: String,
}

#[beet::main]
async fn main() {
	let mut thread = ThreadMut::new();
	let mut human = thread.insert_actor(Actor::human());
	human.insert_post(
		"Tell me about flim-flams, expand on the information given from tool calls, filling in the blanks.",
	);
	let mut agent = thread.insert_actor(Actor::agent());
	agent
		.with_bundle(OllamaProvider::qwen())
		.with_child(function_tool(
			"all-about-flim-flams",
			"learn all about things like flim-flams",
			func_tool(|_cx: FuncToolIn<DiscoverThingsInput>| {
				"flimflams are used by gzorps on their way to work"
					.to_string()
					.xok()
			}),
		));


	println!("1. Make tool call");
	agent
		.send_and_collect()
		.await
		.unwrap()
		.into_iter()
		.for_each(print_post);
	println!("2. read tool output");
	agent
		.send_and_collect()
		.await
		.unwrap()
		.into_iter()
		.for_each(print_post);
}
fn print_post(post: Post) {
	println!("Post: {post}");
}
