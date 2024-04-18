# Concepts
<!-- keep all code references in sync with docs please -->

## Everything is an `Action`

Actions are simply a combination of a `Component` and a `System`. They are the single primitive from which all behaviors are built, whether modifying the world or behavior lifecycles.

For the goal of modularity they are usually very simple, for example `Translate` and `InsertInDuration(RunResult::Success)` could be combined to create a `Translate For Duration` behavior.

## Terminology

These terms may be helpful when describing Beet principles.

| Name     | Descrition                                                                    | Example                                   |
| -------- | ----------------------------------------------------------------------------- | ----------------------------------------- |
| Agent    | A entity that has an associated `Behavior`, usually as a child entity         | `Dragon`                                  |
| Behavior | An entity that contains at least one `Action`, and possibly child `Behaviors` | `Fire Attack`                             |
| Action   | A component-system pair                                                       | `SetAgentOnRun(Animation::BreathingFire)` |

## Graph Roles

Actions can be categorized by what parts of the scene graph they mutate. This is just metadata, there is no techical barrier to creating a super action that does everything, although its usually best to keep actions specific.

| Name               | Description                                                                      | Example                                      |
| ------------------ | -------------------------------------------------------------------------------- | -------------------------------------------- |
| `GraphRole::Node`  | Modifies some part of the entity it is attached to, ie updating a `Score`        | [`ScoreSteerTarget`][score-steer-target]     |
| `GraphRole::Child` | Modifies child entities, ie adding/removing `Running`                            | [`SequenceSelector`][sequence]               |
| `GraphRole::Agent` | Modifies the associated agent, ie movement                                       | [`Translate`][translate]                     |
| `GraphRole::World` | Modifies entities or resources external to the tree, ie despawning a collectable | [`DespawnSteerTarget`][despawn-steer-target] |

## Common Components

These are are the most frequently used components.

- [`Running`][running] - Indicate this node is currently running.
- [`RunResult`][run-result] - Notify their parent that this node has finished.
- [`Score`][score] - Notify the parent how favourable it would be for this node to run.
- [`RunTimer`][run-timer] - Time since an action started/stopped.

[translate]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/core_module/translate.rs
[score-selector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/score_selector.rs
[sequence]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/sequence_selector.rs
[despawn-steer-target]::https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/despawn_steer_target.rs
[score-steer-target]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/score_steer_target.rs

[running]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L12-L13
[run-result]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L32
[score]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/score.rs#L32
[run-timer]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/run_timer.rs