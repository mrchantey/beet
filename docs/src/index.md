# Beet

Beet is a modular AI behavior library for games and robotics. 

It is built with `bevy_ecs` and applies the battle-tested parent-child relationship to behaviors.
This is proving to be an intuitive workflow to those familiar with ecs, and allows for high levels of modularity and flexibility.

## Quick Links
- [Approach](./overview/approach.md)
- [Concepts](./tutorial/concepts-theory.md)
- [Example](./tutorial/concepts-example.md)
- [Beetmash Web Editor](https://app.beetmash.com/)

## Features

#### 🌈 Multi-paradigm

The flexibility of entity graphs allows us to mix-and-match techniques from different paradigms, ie behavior trees, utility selectors, etc.

#### 🌳 Modular

Using an entity graph unlocks epic flexibility, components and systems are reused anywhere in the graph and graphs can be composed of other graphs.

#### 🐦 Ecosystem friendly

All aspects of the library are simple components and systems, which means no blackboard and easy integration with existing bevy tooling.

#### 🎯 Target Anything

Beet only depends on the lightweight architectural components of the bevy library, ie `bevy_ecs`, which allows it to target multi-core gaming rigs and tiny microcontrollers alike.

#### 🔥 Epic Concurrency

By default all actions are run in parallel non-exclusive systems. This means graph traversals occur on each update of the schedule, which makes unit testing, breakpoints etc a breeze, although it is not always desired, see [drawbacks](#multi-tick).

## Quickstart

```rust
use bevy::prelude::*;
use beet::{
  BeetPlugin, Running, RunResult, 
  SequenceSelector, SetOnRun
};

// actions are a component-system pair
#[Derive(Component, Action)]
#[action(system=log_on_run)]
pub struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>){
	for (action) in query.iter(){
		println!("{}", action.0);
	}
}

fn main(){
  let mut app = App::new();

  // the BeetPlugin adds the systems associated with each action,
  // as well as utility systems that clean up run state
  app.add_plugins(BeetPlugin::<(
    SequenceSelector,
    LogOnRun,
    SetOnRun,
    )>::default());

  // behavior graphs are regular entity hierarchies!
  app
    .world_mut()
    .spawn((SequenceSelector::default(), Running))
    .with_children(|parent|
      parent.spawn((
        LogOnRun("Hello"),
        SetOnRun(RunResult::Success)
      ));
      parent.spawn((
        LogOnRun("World"),
        SetOnRun(RunResult::Success)
      ));
    );
  
  // all actions are run in parallel so each update is a tick

  // 1. Selector chooses first child
  app.update();

  // 2. First child succeeds
  app.update(); 
  // "Hello"

  // 3. Selector chooses second child
  app.update();

  // 4. Second child succeeds
  app.update();
  // "World"

  // 5. Selector succeeeds
  app.update();
}
```

## Drawbacks

#### Indirection

Agents, behaviors and children are seperate entities, which is a potential cache miss and ergonomic painpoint. Its my hope this will largely be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Tick Traversal

By default graph traversals are handled in the next tick which is fine for most cases, but if frame perfect traversals are required it will need to be done manually. I can think of a few workarounds:
- Use a custom schedule and update it manually until traversals are complete
- Arrange and/or duplicate system execution in a specific order
- Hardcode actions into a single system