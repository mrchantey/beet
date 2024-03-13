

## Features

- 🐦 Powered by `bevy_ecs` and `petgraph`
- 🔥 Highly Parallel
- ✍️ No Blackboard
- 🌈 Multi-paradigm
- 🌍 With or without Bevy Engine
- 🌴 Create/edit graphs at runtime
- 🧩 Multiple graphs per entity


## Overview

This is my fourth attempt at a modular AI architecture for ECS, the previous two attempts went the way of the dodo:
1. Shoehorn non-ecs solutions into bevy, which sucked mostly because of blackboards. 
2. Get clever with generics and create distinct types *per node* of a graph. This allowed for an entire graph to be stored as components on a single entity but was not great for a bunch of reasons. The dealbreaker was not being able to create/edit graphs at runtime.
3. Create a `petgraph` of `Actions` then convert that into a prefab world


I'm quite confident in this fourth, representing graphs as linked entities. 

### Actions

Actions without children usually execute some behavior then return a `RunResult::Success` or a `RunResult::Failure`.

An `action` is a bevy component struct with an associated system. Currently all actions must implement `Default, Clone, Component, serde::Serialize, serde::Deserialize`.

```rust
#[action(system=print_action)]
#[derive(Default)]
pub struct PrintAction(pub value: String);

fn print_action(mut commands: Commands, query: Query<(Entity,&PrintAction), With<Running>){
	for (entity, action) in query.iter(){
		println!("Print Action: {}", action.0);
		commands.entity(entity).insert(RunResult::Success);
	}
}
```

### Next Steps

Documentation is WIP, in the meantime have a look at `./crates/gamai/test/selectors` for examples of selectors and how they are used.