# `beet_flow`

Beet Flow is an ECS control flow library built with [Bevy Observers][bevy-observers]. The ECS architecture allows for a growing list of paradigms to be used interchangably:
- [Behavior Trees](../../examples/flow/hello_world.rs)
- [Long Running](../../examples/flow/long_running.rs)
- [State Machines](../../examples/flow/state_machine.rs)
- [Utility AI](../../examples/flow/utility_ai.rs)
- [LLM Sentence Similarity](../../examples/ml/hello_ml.rs)
- [Reinforcement Learning](../../examples/ml/frozen_lake_train.rs)

## Hello World

A demonstration of a Sequence control flow common in behavior trees.

```rust
use beet_flow::prelude::*;
use beet_core::prelude::*;

let mut app = App::new();
app.add_plugins((
	// manages action lifecycles
	BeetFlowPlugin::default(),
	// this will log the name of each action as it is triggered.
	BeetDebugPlugin::default()
));

app.world_mut()
	.spawn((
		Name::new("My Behavior"),
		Sequence,
		children![
			(
				Name::new("Hello"),
				EndOnRun::success(),
			),
			(
				Name::new("World"),
				EndOnRun::success(),
			),
		],
	))
	.trigger_entity(RUN)
	.flush();
```

[bevy-observers]:https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html#
