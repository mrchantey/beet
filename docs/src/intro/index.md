# Beet

Beet is a very flexible behavior library for games and robotics.

It is built with `bevy` and represents behaviors as entities, connecting them through the parent-child relationship.

> This library is experimental and I'd love to hear any questions or feedback, my handle is `@mrchantey` on the Bevy Discord.

## Quick Links

- [Concepts](./concepts.md)
- [Actions](./actions.md)
- [Robotics](./robotics.md)

## Examples
- [Hello World](../examples/hello_world.md)
- [Seek](../examples/seek.md)
- [Flocking](../examples/flock.md)

## Features

#### 🌈 Multi-Paradigm

Create behaviors from a growing list of paradigms, check out the [roadmap](./misc/roadmap.md) for implementation status.

#### 🐦 Bevy Friendly

Actions are simply component-system pairs, which means no blackboard and easy integration with the bevy ecosystem.

#### 🕑 Tick Tock

Ticks are ecs-first, running all action systems in parallel. Behavior lifecycles are managed through component changes.

<!-- #### 🌳  -->

#### 🎯 Target Anything

Beet is suitable for powerful gaming rigs and tiny microcontrollers alike.

<!-- #### 🌐 Zero-config replication

Work can be distributed across environments through world replication. An agent may run some actions in a constrained environment and others in a remote server. -->

## Drawbacks

#### Relations

Agents and behaviors are seperate entities requiring their own queries. This may be addressed by the introduction of [Entity Relations](https://github.com/bevyengine/bevy/issues/3742).

#### Tick Traversal

When using the `Update` schedule graph traversals are handled in the next frame, if frame perfect traversals are required there are a couple of options:
- Use a custom schedule and update it manually until traversals are complete
- Arrange and/or duplicate system execution in a specific order
- Hardcode action sequences into a single system