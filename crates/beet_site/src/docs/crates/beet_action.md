+++
title = "beet_action"
+++

# beet_action

In `beet_action`, a function is just an entity you can call.

Attach an [`Action`] component to an entity and it becomes callable. `call` runs the handler with an input and awaits its output. Because the action lives on an entity, it can have children, and control-flow nodes like `Sequence` are nothing more than actions that call their children in turn. Behavior trees, state machines and utility AI are all built from that single primitive.

```rust
# use beet_core::prelude::*;
# use beet_action::prelude::*;
# async fn run() -> Result {
let outcome = AsyncPlugin::world()
	.spawn((Sequence::new(), children![
		Log::new("hello"),
		Log::new("world"),
	]))
	.call::<(), Outcome>(())
	.await?;
# Ok(()) }
```

An action comes in one of three flavors, chosen with the `#[action]` macro to match how much power the handler needs:

- `#[action(pure)]` runs synchronously with no world access. It is the fast path for plain computation.
- `#[action]` on a regular function makes it a Bevy system, taking its input as `In<T>` and reaching for any system param.
- `#[action]` on an `async fn` gets async world access, so the handler can `await` other actions, IO or timers.

The range matters: the same calling convention covers a pure adder and an agent that awaits a network reply, so higher-level crates like `beet_net`, `beet_router` and `beet_thread` can all speak in actions without caring which flavor sits underneath.
