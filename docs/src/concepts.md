# Concepts
<!-- keep all code references in sync with docs please -->

## Everything is an `Action`

Actions are simply a combination of a `Component` and a `System`. They are the single primitive from which all behaviors are built, whether modifying the world or the behavior graph.

For the goal of modularity they are usually very simple, for example `Translate` and `InsertInDuration<RunResult>` could be combined to create a `Translate For Duration` behavior.

## Terminology

These terms are not nessecarily types in the codebase but may be helpful when describing Beet principles.

| Name     | Descrition                                                                | Example         |
| -------- | ------------------------------------------------------------------------- | --------------- |
| Agent    | A entity that has an associated behavior, usually as a child entity       | `Enemy`         |
| Behavior | An entity that contains at least one action, and possibly child behaviors | `Attack Target` |
| Action   | A component-system pair                                                   | `Swing Sword`   |

## Graph Roles

Actions can be categorized by what parts of the scene graph they mutate. This is just metadata and has no actual effect.

| Name               | Description                                                                                                                                 | Example                                      |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `GraphRole::Node`  | Only modifies some part of the behavior entity it is attached to, ie updating `Score` to be interpreted a [`ScoreSelector`][score-selector] | [`ScoreSteerTarget`][score-steer-target]     |
| `GraphRole::Child` | Modifies child entities, ie adding/removing `Running`                                                                                       | [`SequenceSelector`][sequence]               |
| `GraphRole::Agent` | Modifies the associated agent, ie movement                                                                                                  | [`Translate`][translate]                     |
| `GraphRole::World` | Modifies entities or resources external to the tree, ie despawning a collectable                                                            | [`DespawnSteerTarget`][despawn-steer-target] |

## Common Components

These components are used by actions to determine run-state and make decisions.

- [`Running`][running] - Indicate this node is currently running.
- [`RunResult`][run-result] - Notify their parent that this node has finished.
- [`Score`][score] - Notify the parent how favourable it would be for this node to run.
- [`RunTimer`][run-timer] - Time since an action started/stopped.
- [`InsertOnRun<T>`][insert-on-run] - Inserts a component whenever state changes


[translate]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/core_module/translate.rs
[score-selector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/score_selector.rs
[sequence]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/sequence_selector.rs
[despawn-steer-target]::https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/despawn_steer_target.rs
[score-steer-target]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/score_steer_target.rs
[insert-on-run]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/ecs_module/actions/lifecycle_actions.rs#L41

[running]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L12-L13
[run-result]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L32
[score]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/score.rs#L32
[run-timer]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/run_timer.rs