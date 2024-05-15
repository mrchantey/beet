# Concepts
<!-- keep all code references in sync with docs please -->

## Everything is an `Action`

Actions are simply a combination of a `Component` and a `System`. They are the single primitive from which all behaviors are built, whether modifying the world or traversing the behavior graph.

For the purpose of modularity they are usually very simple, for example `Translate` and `InsertInDuration(RunResult::Success)` could be combined to create a `Translate For Duration` behavior.

## Terminology

These terms may be helpful when describing Beet principles.

| Name     | Descrition                                                                    | Example                                  |
| -------- | ----------------------------------------------------------------------------- | ---------------------------------------- |
| Agent    | A entity that has an associated `Behavior`, usually as a child entity         | Dragon                                   |
| Behavior | An entity that contains at least one `Action`, and possibly child `Behaviors` | Fire Attack                              |
| Action   | A component-system pair                                                       | Set agent's animation to `BreathingFire` |

## Action Category

Action categories describe what the action modifies. This is just metadata, there is no techical barrier to creating a super action that makes lots of changes, although keeping actions specific allows for more code reusability.

| Name                             | Description                                                            | Example                                      |
| -------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------- |
| `ActionCategory::Behavior`       | Modifies some part of its own behavior, like setting the `RunResult`   | [`InsertOnRun<RunResult>`][run-result]       |
| `ActionCategory::ChildBehaviors` | Modifies child behaviors, like adding/removing `Running`               | [`SequenceSelector`][sequence]               |
| `ActionCategory::Agent`          | Modifies the associated agent, like its `Transform`                    | [`Translate`][translate]                     |
| `ActionCategory::World`          | Modifies external entities or resources, like despawning a collectable | [`DespawnSteerTarget`][despawn-steer-target] |

## Common Components

Here are some of the most frequently used components.

- [`Running`][running] - Indicate this node is currently running.
- [`RunResult`][run-result] - Added by actions to notify their parent that they have finished.
- [`Score`][score] - Notify the parent how favourable it would be for this node to run.
- [`RunTimer`][run-timer] - Time since an action started/stopped.
- `RootIsTargetAgent` - Before the first tick, this will be replaced by a `TargetAgent` pointing to the root of its hierarchy.

[translate]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/core_module/translate.rs
[score-selector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/score_selector.rs
[sequence]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/sequence_selector.rs
[despawn-steer-target]::https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/despawn_steer_target.rs
[score-steer-target]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steering/steering_actions/score_steer_target.rs

[running]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L12-L13
[run-result]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/running.rs#L32
[score]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/score.rs#L32
[run-timer]:https://github.com/mrchantey/beet/blob/84047347bd0f1ca371503718d5cb0a0dd265709f/crates/beet_ecs/src/node/run_timer.rs
