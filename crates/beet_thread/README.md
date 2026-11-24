# beet_thread

Conversations between actors, kept as records.

A [`Thread`] is a conversation: [`Post`]s by [`Actor`]s, people, agents or the system on equal footing, kept in a store. People post to each other with no agent around, and an agent is one more actor whose turn is an action, so the same machinery is a group chat, a person working with an agent or a fleet of agents working together. Every word of the model, from thread and seed to window, session, turn, the three shapes and lineage, is defined once in the [`table`] module docs.

A thread entity is a window onto one stored thread, named by its store path: `<Thread {(Sequence, MountThreadStore{path:"threads/chat"})}>`. A [`RunThread`] holds one for a runtime's life, and a [`ThreadRoute`] runs one as a function, a request in and an answer out. An agent's tools are the routes of its own [`Router`], each named by its route path, and a [`RouteTool`] makes a tool of a route in the surrounding url space.

Providers: OpenAI, Gemini, Bedrock, Ollama, plus a mock provider for tests. Replies can be streamed or collected.

```rust
use beet_core::prelude::*;
use beet_thread::prelude::*;

# async_ext::block_on(async {
let replies = Post::run_oneshot(children![
	(Actor::user(), children![Post::spawn("hi")]),
	(Actor::agent(), MockPostStreamer::default()),
])
.await
.unwrap();
replies[0].to_string().xpect_eq("you said: hi");
# });
```

[`Thread`]: crate::prelude::Thread
[`Post`]: crate::prelude::Post
[`Actor`]: crate::prelude::Actor
[`RunThread`]: crate::prelude::RunThread
[`ThreadRoute`]: crate::prelude::ThreadRoute
[`Router`]: beet_router::prelude::Router
[`RouteTool`]: crate::prelude::RouteTool
[`table`]: crate::table
