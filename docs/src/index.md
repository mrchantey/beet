# Beet

Beet is a very flexible behavior library for games and robotics. It is built with `bevy` and represents behaviors as entities, connecting them through the parent-child relationship.

> This library is experimental and I'd love to hear any questions or feedback, my handle is `@mrchantey` on the Bevy Discord.

## Quick Links

- [Concepts](intro/concepts.md)
- [Actions](intro/actions.md)
- [Examples](examples/index.md)

## Features

#### 🌈 Multi-Paradigm

Interoperate between Behavior Trees, Utility AI, Reinforcement Learning, and other behavior paradigms, check out the [roadmap](misc/roadmap.md) for implementation status.

#### 🌳 Modular

Actions are very simple and entity trees are self-contained, enabling behavior composition.

#### 🎯 Target Anything

Runs on servers, web, mobile and even tiny microcontrollers like the ESP32.


### 🌐 Zero-config replication

Rendering, sensor input & decision-making can be distributed across devices through simple world replication.

#### 🐦 Bevy Friendly

Beet is regular components, systems and plugins all the way down. Behaviors can be visualized, serialized etc in the same way as bevy scenes.

### 🕯️ Machine Learning

100% Rust RL environments with [Huggingface Candle](https://github.com/huggingface/candle) integration, including rust ports of OpenAI Gym environments like `Frozen Lake`.

## Drawbacks

#### Relations

Agents and behaviors are seperate entities requiring their own queries. This may be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Tick Traversal

By default all actions run concurrently meaning graph traversals are handled in the next frame. If single frame traversals are required there are a couple of options:
- Use a custom schedule and update it manually until traversals are complete
- Arrange and/or duplicate system execution in a specific order
- Hardcode action sequences into a single system
