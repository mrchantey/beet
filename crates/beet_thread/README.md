# beet_thread

Multi-actor conversation orchestration.

A [`Thread`] is a sequence of [`Actor`]s, each owning their [`Post`]s. An actor may be a person, a system prompt or an LLM agent, and they all participate on equal footing, so the same machinery drives a plain group chat, a human-in-the-loop workflow or a fleet of cooperating agents.

Running a thread advances the conversation: an agent actor sends the transcript to its provider and appends the reply. Because actors are entities, an agent's tools are just the routes of the [`Router`] beneath it, its own url space: a tool's name is its route path, and a call of anything else is that router's not-found. A tool is a route taking a typed input, its arguments that input sent as a JSON body. A verb of the url space hosting the agent becomes one of its tools as a [`HostTool`], `<HostTool path="eval/grade"/>`, a route at the same path whose definition is read off the host's route, its declared params the arguments, and whose call is forwarded there with the arguments as those params, so the host validates them as it does flags, its pinned params (`params="table=claims"`) out of the agent's reach. A multi-agent setup is just more actors in the tree.

A thread entity is one thread for its whole life. Its seed is resolved once while it builds, a [`SeedSource`] (`<CreatePost src="..">`, or `<RoutePost path="..">`, the answer of a route of the url space the thread is attached in) read as part of the build, and the seed hash is its identity: [`Thread::enter`] adopts the stored thread sharing it, so a reload resumes. Another seed is another thread entity, ie a [`ThreadRoute`] building one per request, `<Route path="review/:document" {ThreadRoute{template:"ReviewThread"}}/>`, whose seed reads its path params.

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

Add [`ThreadStdoutPlugin`] to stream messages to the terminal, and spawn a [`Router`] of `route::exchange`s beneath an agent actor to give it tools.
