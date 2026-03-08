Time to prepare for integrating `bevy_malek_async` as our async runtime.

have a thorough read of `agent/reference/bevy_malek_async`, particularly `examples/utils` where I've started workshopping how to integrate with our existing `crates/beet_core/src/bevy_utils/async_commands.rs`

The current approach uses channels *everywhere* including awaiting a task to be done like `AsyncWorld::with_then`. This new approach seems to need only a `WorldId`.

Lets start by refactoring AsyncWorld to only need a WorldId. We're not ready to throw out the channels approach, `bevy_malek_async` is very much unproven so we may need to toggle it on and off. perhaps we can use some kind of static LazyLock instead of AsyncChannel, it will need some design work.

Hopefully the refactor will reduce or eliminate the hard dependency on channels, turning it instead into an alternative to `malek_async`.

I'm not sure what the implications will be for AsyncRunner. The current impl is just a first draft and a little messy, make changes as needed.

Note that `bevy_malek_async` is agnostic to the task spawning, for now we will still use `IoTaskPool` but it could be worth seperating that out so its easier to swap out the async runtime. Again that may need some design work.

The criteria for success is to be able to toggle between channels and the new system with a feature flag: `malek_async`, which adds the crate as a dependency with a path `../../agent/reference/bevy_malek_async` be sure to use timeout when running tests theres a high likelihood of hanging. Hopefully we wont need to change `bevy_malek_async` to get it working, if so make the changes clear in the summary.


test wasm too
`cargo test -p beet_core --lib | tail`
`cargo test -p beet_core --lib --target wasm32-unknown-unknown | tail`
