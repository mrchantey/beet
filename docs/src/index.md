# Beet

> This library has a broad scope and is largely incomplete, contributions are very welcome.

Beet is a modular behavior library for games and robotics.

It is built with `bevy` and uses entities to represent behaviors, connecting them through the parent-child relationship. This approach feels familiar and allows for high levels of modularity, just like scene graphs (in fact behaviors *are* scene graphs).

## Quick Links

- [Concepts](./concepts.md)
- [Actions](./actions.md)

## Features

#### 🌈 Multi-paradigm

Actions can be created for any paradigm, from behavior trees to llms and everything in between.

#### 🌳 Modular Trees

Behaviors are composable trees of entities and actions are reusable.

#### 🐦 Ecosystem friendly

All aspects of the library are simple components and systems, which means no blackboard and easy integration with existing bevy tooling.

#### 🎯 Target Anything

Beet only depends on the lightweight architectural components of the bevy library, ie `bevy_ecs`, which allows it to target multi-core gaming rigs and tiny microcontrollers alike.

#### 🔥 Epic Concurrency

By default all actions are run in parallel systems. This means graph traversals occur on each update of the schedule, which makes unit testing, breakpoints etc a breeze, although it is not always desired, see [drawbacks](#multi-tick).

<!-- #### 🌐 Zero-config replication

Work can be distributed across environments through world replication. An agent may run some actions in a constrained environment and others in a remote server. -->

## Quickstart

```rust
use beet::prelude::*;
use bevy::prelude::*;

// actions are a component-system pair
// by default the system is the StructName in snake_case
#[derive(Component, Action)]
pub struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for action in query.iter() {
		println!("{}", action.0);
	}
}

fn main() {
	let mut app = App::new();

	// the BeetSystemsPlugin adds each action system
	// and some helpers that clean up run state
	app.add_plugins(BeetSystemsPlugin::<(
      SequenceSelector, 
      InsertOnRun<RunResult>
      LogOnRun, 
    ), Update>::default());

	// behavior graphs are regular entity hierarchies
	app.world_mut()
		.spawn((SequenceSelector::default(), Running))
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

	// each update is a tick

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

#### Lots of Relations

Agents, behaviors and children are seperate entities, which can be an ergonomic painpoint. Hopefully this will be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Tick Traversal

By default graph traversals are handled in the next tick which is fine for most cases, but if frame perfect traversals are required it will need to be done manually. I can think of a few workarounds:
- Use a custom schedule and update it manually until traversals are complete
- Arrange and/or duplicate system execution in a specific order
- Hardcode actions into a single system