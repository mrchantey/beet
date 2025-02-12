# `beet_flow`

Beet Flow is an ECS control flow library, built with [Bevy Observers][bevy-observers]. Being ECS first gives Beet a level of flexibility and modularity not usually found in control flow libraries.

Currently implemented paradigms:
- [Behavior Trees](../../examples/flow/hello_world.rs)
- [Long Running](../../examples/flow/long_running.rs)
- [State Machines](../../examples/flow/state_machine.rs)
- [Utility AI](../../examples/flow/utility_ai.rs)
<!-- - [LLM Sentence Similarity](../../examples/hello_ml.rs)
- [Reinforcement Learning](../../examples/frozen_lake_train.rs) -->

## Hello World

```rust
use bevy::prelude::*;
use beet::prelude::*;

// A demonstration of Sequence control flow
world.spawn(SequenceFlow)
	.with_child((
		Name::new("Hello"),
		EndOnRun::success(),
	))
	.with_child((
		Name::new("World"),
		EndOnRun::success(),
	))
	.trigger(OnRun);
```

[bevy-observers]:https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html#


## TODO

- When we get [`OnMutate`](https://github.com/bevyengine/bevy/pull/14520) observers, they should probably replace most `OnInsert` observers we're using
