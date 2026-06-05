+++
title = "beet_thread"
+++

# beet_thread

`beet_thread` orchestrates conversations between many actors. A `Thread` is a sequence of `Actor`s, each owning their `Post`s, and an actor might be a person, a system prompt or an LLM agent. They all participate on equal footing, so the same machinery drives a plain group chat, a human-in-the-loop workflow or a fleet of cooperating agents.

Running a thread advances the conversation: an agent actor sends the transcript to its provider and appends the reply. Because actors are entities and tools are just child [`Action`] routes, giving an agent a capability means spawning another route on it, and a multi-agent system is simply more actors in the tree, not a different architecture.

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

Providers include OpenAI, Gemini, Bedrock and Ollama, plus a mock provider for tests, and replies can be streamed or collected in one go. Add `ThreadStdoutPlugin` to stream a conversation to the terminal.
