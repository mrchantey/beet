# beet_thread

Multi-actor conversation orchestration.

A [`Thread`] is a sequence of [`Actor`]s, each owning their [`Post`]s. An actor may be a person, a system prompt or an LLM agent, and they all participate on equal footing, so the same machinery drives a plain group chat, a human-in-the-loop workflow or a fleet of cooperating agents.

Running a thread advances the conversation: an agent actor sends the transcript to its provider and appends the reply. Because actors are entities, tools are just child [`Action`] routes an agent can call, and a multi-agent setup is just more actors in the tree.

Providers: OpenAI, Gemini, Bedrock, Ollama, plus a mock provider for tests. Replies can be streamed or collected.

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
		.with_bundle(OpenAiProvider::gpt_5_4_mini().unwrap())
		.send_and_collect()
		.await
		.unwrap();

	println!("{posts:#?}");
}
```

Add [`ThreadStdoutPlugin`] to stream messages to the terminal, and spawn child `exchange_route`s on an agent actor to give it tools.
