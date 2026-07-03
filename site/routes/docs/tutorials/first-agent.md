+++
title = "Your first agent"
+++

# Your first agent

In this tutorial we will hold a short conversation with an LLM. By the end you will have sent a message to an agent and printed its reply, using the same actor-and-route machinery that drives beet's larger multi-agent setups.

This tutorial talks to OpenAI, so you will need an API key from your OpenAI account.

## Set up the project

Create a new binary crate and add beet with the `thread` feature:

```sh
cargo new hello-agent
cd hello-agent
cargo add beet --features thread
```

Put your API key in a `.env` file next to `Cargo.toml`:

```sh
OPENAI_API_KEY=sk-your-key-here
```

## Write the conversation

Open `src/main.rs` and replace its contents with this:

```rust
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

We start a thread, add a system actor that sets the instruction, then add an agent actor backed by an OpenAI model. `send_and_collect` sends the transcript and waits for the whole reply.

## Run it

```sh
cargo run
```

After the build, the program reaches out to OpenAI and prints the posts in the conversation, ending with the agent's reply. You should see something duck-like:

```text
Quack!
```

Notice that both your instruction and the reply come back as `Post`s. The conversation is data, not a one-shot call, which is what lets a thread grow.

## Change the prompt

Edit the system post to ask for something else, say `"reply only in haiku"`, and run again. The agent follows the new instruction. Remember that the agent actor is just another participant in the thread, on equal footing with the system and, in larger setups, with other agents and people.

## What you have built

You have run a conversation with an LLM agent. Because the agent is an entity, giving it tools means spawning routes on it, and adding a second agent means adding another actor, the same patterns you met in the earlier tutorials. From here, the [Crates](/docs/crates) section explains how these pieces fit together across the whole engine.
