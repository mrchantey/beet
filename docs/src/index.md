# Beet

Beet is a very flexible behavior library for games and robotics.

It is built with `bevy` and represents behaviors as regular entities, connecting them through the parent-child relationship.

> This library is experimental, if you have any questions or feedback my Bevy discord handle is `@mrchantey`.

## Quick Links

- [Concepts](./concepts.md)
- [Actions](./actions.md)
- [Robotics](./robotics.md)

## Features

#### 🌈 Multi-Paradigm

Create behaviors from a growing list of paradigms, check out the [roadmap](./misc/roadmap.md) for implementation status.

#### 🐦 Bevy Friendly

Actions are simply component-system pairs, which means no blackboard and easy integration with existing bevy tooling.

#### 🕑 Tick Tock

Ticks are ecs-first, running all action systems in parallel. Behavior lifecycles are managed through component changes.

<!-- #### 🌳  -->

#### 🎯 Target Anything

Beet is suitable for powerful gaming rigs and tiny microcontrollers alike.

<!-- #### 🌐 Zero-config replication

Work can be distributed across environments through world replication. An agent may run some actions in a constrained environment and others in a remote server. -->

## Quickstart

In this example we will create an action and then use it with some built-in actions to run a behavior.

```rust
use beet::prelude::*;
use bevy::prelude::*;

// actions are a component-system pair
// by default the system is the ComponentName in snake_case
#[derive(Component, Action)]
pub struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for action in query.iter() {
		println!("{}", action.0);
	}
}

fn main() {
	let mut app = App::new();

	app.add_plugins(
		// This plugin schedules every action in parallel
		BeetSystemsPlugin::<(
			SequenceSelector, 
			InsertOnRun<RunResult>
			LogOnRun, 
		), 
		// Define your own schedule to run ticks independently
		Update
		>::default());

	// Behaviors are regular entity hierarchies
	app.world_mut()
		.spawn((
			// start this behavior running at its root
			Running, 
			SequenceSelector::default(), 
		))
		.with_children(|parent| {
			parent.spawn((
				LogOnRun("Hello".into()),
				InsertOnRun(RunResult::Success),
			));
			parent.spawn((
				LogOnRun("World".into()),
				InsertOnRun(RunResult::Success),
			));
		});

	// graph traversals occur on each tick
	println!("1 - Selector chooses first child");
	app.update();

	println!("2 - First child runs");
	app.update();

	println!("3 - Selector chooses second child");
	app.update();

	println!("4 - Second child runs");
	app.update();

	println!("5 - Selector succeeds, all done");
	app.update();
}
```
```
cargo run --example hello_world

1 - Selector chooses first child
2 - First child runs
Hello
3 - Selector chooses second child
4 - Second child runs
World
5 - Selector succeeds, all done
```


## Drawbacks

#### Relations

Agents and behaviors are seperate entities requiring their own queries. This may be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Tick Traversal

When using the `Update` schedule graph traversals are handled in the next frame, if frame perfect traversals are required there are a couple of options:
- Use a custom schedule and update it manually until traversals are complete
- Arrange and/or duplicate system execution in a specific order
- Hardcode action sequences into a single system