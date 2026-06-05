# beet_thread

LLM conversations and agents, modeled as entity hierarchies.

A [`Thread`] is a sequence of [`Actor`]s (system, user, agent), each owning their [`Post`]s. Running the thread sends the conversation to an agent's provider and appends the reply. Because actors are entities, tools are just child [`Action`] routes the agent can call, and multi-agent setups are just more actors in the tree.

Providers: OpenAI, Gemini, Bedrock, Ollama, plus a mock provider for tests. Responses can be streamed or collected.

```rust,ignore
use beet::prelude::*;

#[beet::main]
async fn main() {
	env_ext::load_dotenv();

	let posts = ThreadMut::spawn()
		.insert_actor(Actor::system())
		.insert_post("make like a duck and quack")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(OpenAiProvider::gpt_5_mini().unwrap())
		.send_and_collect()
		.await
		.unwrap();

	println!("{posts:#?}");
}
```

Add [`ThreadStdoutPlugin`] to stream agent messages to the terminal, and spawn child `exchange_route`s on an agent actor to give it tools.
