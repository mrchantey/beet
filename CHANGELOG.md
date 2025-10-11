# Changelog
## 0.0.8

### `beet_flow`
- `BeetFlowPlugin` -> `ControlFlowPlugin`



## 0.0.7

### `beet_flow`

Bevy had an [`Event Overhaul`] in 0.17 so naturally a lot has shifted around in this release, I've used the opportunity to dramatically simplify the api.

#### [`AgentQuery`]

The introduction of global observers led to a mess of tracking the `origin` (now referred to as `agent`) through `OnRun`, `Running` and `OnResult` calls, leading to dreadfully complex triggering. Tracking the `origin` is now done through an `AgentQuery`, see its documentation for more details.

####  [`GetOutcome`] / [`Outcome`]

`OnRun` / `OnResult` has been refactored to better suit the direction of events in bevy, now with a request/response `Foo` / `GetFoo` convention. `GetOutcome` is longer to type than `Run` but clearly defines the relationship with `Outcome` and is much easier to grep in the codebase. Likewise we now have `GetScore` / `Score`.

#### Removed: Global Observers

I still think this pattern may be useful if you have >1000 entities frequently added and removed, although in that case something like a pool might be more effective. However the juice aint worth the squeeze for the 90% of other cases, removing this makes tha api a lot simpler to reason about. Also there is no reason for this pattern to be built in, just add a `Run` global observer.


### `beet_net`

- Added support for Sockets/SSE.
- DynamoDB Integration
- Basic analytics

### `beet_rsx`
- Print to pdf helpers
- Template refactor, prep for bevy 0.18

### `beet_dom`
- Webdriver BiDi
- Pdf exports


## 0.0.6

Added web related crates.


## 0.0.5

### `beet_flow`

#### Global Observers

Beet is going global! Using global action observers provides several performance and usability improvements:
- 5x faster action spawning/despawning
- `OnRun` now carries the action that triggered it and what im calling `origin`, aka agent. I'm avoiding using that term because `beet_flow` is a general purpose control flow library.
> Warning! if you were relying on `trigger.entity()` in an `OnRun` observer, that will now point to the global observer, instead use `trigger.action`


#### Automatic Action Adding

Both as a usability improvement and to facilitate global observer spawning, the `#[action]` macro
now creates global observers in the `on_add` component hook. This means that `#[systems]` and `#[global_observers]` attributes are deprecated as we no longer have access to `App`, please add them like you would in vanilla bevy.

### Changed
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

### Removed
- `ActionPlugin`: observers now automatically added
- `TargetEntity`: Use `OnRun::origin`
- `RootIsTargetEntity`: Use `OnRun::origin`
- `DynamicRootIsTargetEntity`: Use `OnRun::origin`
