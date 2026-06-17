+++
title = "beet_async"
+++

# beet_async

> ⚠️ Temporary crate. `beet_async` is a beet-owned vendored copy of the in-progress upstream `bevy_async`, rewired onto beet's single `bevy` dependency. It will be dropped once upstream lands the two features beet needs: exclusive world access and wasm support.

`beet_async` solves one sharp problem: letting two parties share `&mut World`. On one side is the main Bevy schedule, on the other are futures and async tasks running on different threads.

The trick is cooperation rather than locking around every access. A future asks for the world, the `async_world_sync_point` driver system hands it exclusive access for a scope, then normal scheduling resumes. That is the primitive [beet_action](/docs/crates/beet_action) is built on, which is why most code never touches beet_async directly and instead reaches it through `AsyncPlugin` and `AsyncWorld`.

It earns its own crate mostly because exclusive world access across threads is subtle, and the locking protocol and invariants need a single documented home. Treat it as plumbing: important that it works, rarely something you handle yourself.
