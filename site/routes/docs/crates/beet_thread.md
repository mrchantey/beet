+++
title = "beet_thread"

[RouteMeta]
visibility = "unlisted"
+++

# beet_thread

`beet_thread` keeps conversations between actors. A `Thread` is posts by `Actor`s, people, agents or the system on equal footing, kept as rows in a table store, so a thread of people alone needs nothing an agent brings and an agent is one more participant whose turn is an action.

Because a turn is an action, a thread's behavior is ordinary [beet_action](/docs/crates/beet_action) control flow: a `Sequence` takes turns one at a time, a `Parallel` lets actors speak at once. An agent's tools are the routes of its own `Router`, so giving it a capability means mounting another route, and a multi-agent system is more actors in the tree rather than a different architecture.

Every word of the model, from seed and session to window, turn, editing, compaction and lineage, is defined once in the crate's `table` module docs.

```rust
# use beet_core::prelude::*;
# use beet_thread::prelude::*;
# async fn run() -> Result {
let replies = Post::run_oneshot(children![
	(Actor::user(), children![Post::spawn("hi")]),
	(Actor::agent(), MockPostStreamer::default()),
])
.await?;
# Ok(())
# }
```

Providers include OpenAI, Gemini, Bedrock and Ollama, plus a mock provider for tests, and replies can be streamed or collected in one go.
