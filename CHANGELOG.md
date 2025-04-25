
# 0.0.5

## `beet_flow`

### Global Observers

Beet is going global! Using global action observers provides several performance and usability improvements:
- 5x faster action spawning/despawning
- `OnRun` now carries the action that triggered it and what im calling `origin`, aka agent. I'm avoiding using that term because `beet_flow` is a general purpose control flow library.
> Warning! if you were relying on `trigger.entity()` in an `OnRun` observer, that will now point to the global observer, instead use `trigger.action`


### Automatic Action Adding

Both as a usability improvement and to facilitate global observer spawning, the `#[action]` macro
now creates global observers in the `on_add` component hook. This means that `#[systems]` and `#[global_observers]` attributes are deprecated as we no longer have access to `App`, please add them like you would in vanilla bevy.

## Changed
- `ParallelFlow` now awaits all child results before returning, if any fail it will fail immediately.

- The `Flow` prefix has been replaced by [ActionTag], used as a convention in the docs:
	```rust
	/// Does cool stuff
	/// ## Tags:
	/// - [`ControlFlow`](ActionTag::ControlFlow)
	/// - [`LongRunning`](ActionTag::LongRunning)
	/// ## Example
	/// ...
	struct MyAction;
	```

## Removed
- `ActionPlugin`: observers now automatically added
- `TargetEntity`: Use `OnRun::origin`
- `RootIsTargetEntity`: Use `OnRun::origin`
- `DynamicRootIsTargetEntity`: Use `OnRun::origin`