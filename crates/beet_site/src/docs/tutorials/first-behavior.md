+++
title = "A first behavior"
+++

# A first behavior

In this tutorial we will build a tiny behavior tree and run it to completion. By the end you will have watched an entity call its children in order, which is the seed every larger behavior in beet grows from.

## Set up the project

Create a new binary crate and add beet with its action feature, which brings in the behavior primitives:

```sh
cargo new hello-beet
cd hello-beet
cargo add beet --features action
```

Open `src/main.rs` and replace its contents with this:

```rust
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let outcome = AsyncPlugin::world()
		.spawn((Sequence::new(), children![
			Log::new("hello"),
			Log::new("world"),
		]))
		.call::<(), Outcome>(())
		.await?;

	println!("{outcome:?}");
	Ok(())
}
```

We spawn a single entity with a `Sequence` action and two `Log` children, then `call` it. Notice that `main` is an ordinary `async fn`: the `#[beet::main]` attribute sets up the runtime so we can `await` the call directly.

## Run it

```sh
cargo run
```

The first build pulls in Bevy and will take a moment. When it finishes you should see the two logs in order, followed by the outcome:

```text
hello
world
Success
```

Notice the order. The `Sequence` ran `hello` first and only then `world`, exactly as it was written in the tree. The final `Success` is the `Outcome` the whole entity returned once every child had finished.

## Change the order

Now swap the two `Log` children so `world` comes first, and run again. The logs follow the new order. Remember that the tree *is* the behavior: there is no separate script driving it, only the entity and its children.

## What you have built

You have built and run a behavior tree. The `Sequence` you used is itself just an action that calls its children, so the same `call` you used here scales up to state machines, utility AI and agents. Next, [Speak every interface](/docs/tutorials/every-interface) shows how an action becomes something the outside world can reach.
