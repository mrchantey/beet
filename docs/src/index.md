# Beet

Beet is a modular AI Behavior library that uses a novel `entity graph` approach to behavior selection. It is built with `bevy_ecs` and is suitable for creating autonomous agents in games, simulation and robotics.


<iframe src="https://mrchantey.github.io/beet/play/?spawn-bee=&spawn-flower=&hide-graph=&graph=CAAAAAAAAABOZXcgTm9kZQEAAAAAAAAAAAAAAAAAAD%2FNzMw9AAAAAAAAAAA"></iframe>

## Features

#### 🌈 Multi-paradigm

The flexibility of entity graphs allows us to mix-and-match techniques from different paradigms, ie transitions, utility selectors, etc.

#### 🌳 Modular

Using an entity graph unlocks epic flexibility:
- Components and systems are reused anywhere in the graph and graphs can be composed of other graphs
- Runtime composition
- Multiple graphs per agent
- No-agent graphs

#### 🐦 Ecosystem friendly

All aspects of the library are simple components and systems, which means no blackboard and easy integration with existing bevy tooling.

#### 🎯 Target Anything

Beet only depends on the lightweight architectural components of the bevy library, ie `bevy_ecs`, which allows it to target multi-core gaming rigs and tiny microcontrollers alike.

#### 🔥 Highly Parallel

By default every action system is run in parallel with no built-in sync points.

## Quickstart

```rust
// ## 1. define an action

// actions are just a component-system pair
#[Derive(Component, Action)]
#[action(system=log_on_run)]
pub struct LogOnRun(pub value: String);

fn log_on_run(query: Query<&PrintAction, Added<Running>){
	for (action) in query.iter(){
		println!("{}", action.0);
	}
}


// ## 2. define a graph
let my_graph = BeetNode::new(SequenceSelector)
  .child((
    // any component can be used here
    LogOnRun("Hello"),
    SetOnRun(RunResult::Success)
  ))
  .child((
    LogOnRun("World"), 
    SetOnRun(RunResult::Success)
  ));

// ## 3. Spawn the graph for an agent
let my_agent = world.spawn_empty().id();
my_graph.spawn(world, my_agent);
```

## Drawbacks

#### Indirection

Actions and the agents they work on are seperate queries, which is a potential cache miss and ergonomic painpoint. Its my hope this will be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Multi-tick

Parallelism means graph traversals are addressed in the next tick which is fine for most cases, but if frame perfect ticks are required it will need to be done manually. There are currently two ways of doing this:
- Arrange and/or duplicate the systems in a specific order with `apply_deferred ` sync points
- Use a custom schedule and update it on a loop until all traversals are complete