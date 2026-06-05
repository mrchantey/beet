# beet_action

Functions as entities.

An [`Action`] component turns an entity into a function: `call` runs its handler with an input and awaits the output. Control-flow nodes like [`Sequence`] are just actions that call their children, so behavior trees, state machines and utility AI are all built from the same primitive.

```rust
# use beet_core::prelude::*;
# use beet_action::prelude::*;
# async fn run() -> Result {
// spawn actions as components, then call the entity
let outcome = AsyncPlugin::world()
	.spawn((Sequence::new(), children![
		Log::new("hello"),
		Log::new("world"),
	]))
	.call::<(), Outcome>(())
	.await?;
# Ok(()) }
```

The `#[action]` macro produces an action from a plain function in one of three flavors:

- `#[action(pure)]` runs synchronously with no world access. Fast.
- `#[action]` is a Bevy system: it takes its input as `In<T>` and may use any system param.
- `async fn` with `#[action]` gets async world access, so a handler can `await` other actions, IO or timers.

```rust,ignore
// pure: input in, output out
#[action(pure)]
#[derive(Component)]
fn Add(cx: ActionContext<(i32, i32)>) -> i32 { cx.0 + cx.1 }

// system: world access via system params
#[action]
fn CountNames(_cx: In<()>, names: Query<&Name>) -> usize { names.iter().count() }

// async: await other actions, IO or timers
#[action]
#[derive(Component)]
async fn Greet(cx: ActionContext<String>) -> String {
	format!("Hello, {}!", cx.value())
}
```
