# Beet

Beet is a modular AI Behavior library that uses a novel `entity graph` approach to behavior selection. It is built with `bevy_ecs` and is suitable for creating autonomous agents in games, simulation and robotics.

<iframe src="https://mrchantey.github.io/beet/play/?spawn-bee=&spawn-flower=&hide-graph=&graph=CAAAAAAAAABOZXcgTm9kZQEAAAAAAAAAAAAAAAAAAD%2FNzMw9AAAAAAAAAAA"></iframe>

## Features

### 🌈 Multi-paradigm

The flexibility of entity graphs allows us to mix-and-match techniques from different behavior selection approaches.

### 🌳 Modular

Actions can be reused and graphs can be composed of other graphs, allowing for epic code reusability.

### 🐦 Ecosystem friendly

All aspects of the library are simple components and systems, which makes it incredibly easy to integrate into existing bevy tooling, ie:
- Visualization - `bevy-mod-debugdump`
- Serialization - `bevy_reflect`
- UI - `bevy-inspector-egui`

### 🎯 Target Anything

Beet only depends on the lightweight architectural components of the bevy library, ie `bevy_ecs`, which allows it to target multi-core gaming rigs and tiny microcontrollers alike.

## Drawbacks - Indirection

Actions and the agents they work on are seperate queries, which is a potential cache miss and ergonomic painpoint. Its my hope this will be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).