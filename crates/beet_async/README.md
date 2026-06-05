# beet_async

> ⚠️ Temporary crate ⚠️
>
> A beet-owned vendored copy of the in-progress upstream `bevy_async`, rewired onto beet's single `bevy` dependency. It will be dropped once upstream lands the two features beet needs: exclusive world access and wasm support.

`beet_async` lets two participants share `&mut World`:

- the main Bevy schedule
- futures and async tasks running on other threads

A future asks for the world, the [`async_world_sync_point`] driver system hands it exclusive access for a scope, then resumes normal scheduling. This is the primitive [`beet_action`] is built on, so most users reach it indirectly via [`AsyncPlugin`] and [`AsyncWorld`].

The implementation details, invariants and locking protocol are documented in the crate-level docs below.
