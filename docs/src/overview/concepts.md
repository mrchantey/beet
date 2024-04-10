# Concepts
<!-- keep all code references in sync with docs please -->

The scope of beet spans several AI strategies, many with their own terminology, so I'm doing my best to use terms that fit. If you have any ideas please create an issue.

## Everything is an `Action`

Actions are simply a combination of a `Component` and a `System`. They are the single primitive from which all behaviors are built, whether modifying the world or the behavior graph.

For the goal of modularity they are usually very simple, for example `Translate` and `SucceedInDuration` could be combined to create a `Translate For Duration` behavior.

## Terminology

These terms are not nessecarily types in the codebase but may be helpful when describing Beet principles.

| Name     | Descrition                                                                | Example         |
| -------- | ------------------------------------------------------------------------- | --------------- |
| Agent    | A entity that has an associated behavior, usually as a child              | `Enemy`         |
| Behavior | An entity that contains at least one action, and possibly child behaviors | `Attack Target` |
| Action   | A component-system pair                                                   | `Swing Sword`   |

### Action types
| Name             | Descrition                                                                  | Example                                      |
| ---------------- | --------------------------------------------------------------------------- | -------------------------------------------- |
| Agent Action     | Modifies the associated agent                                               | [`Translate`][translate]                     |
| World Action     | Modifies entities or resources external to the tree                         | [`DespawnSteerTarget`][despawn-steer-target] |
| Selector Action  | Choose which child to run, adds/removes `Running`                           | [`SequenceSelector`][sequence]               |
| Evaluator Action | Modifies the `Score`, to be interpreted a [`ScoreSelector`][score-selector] | [`ScoreSteerTarget`][score-steer-target]     |
| Lifecycle Action | Does something as a reaction to the run state changing                      | [`InsertOnRun<T>`][insert-on-run]            |

## Common Components

These components are used by actions to determine run-state and make decisions.

- [`Running`][running] - Indicate this node is currently running.
- [`RunResult`][run-result] - Notify their parent that this node has finished.
- [`Score`][score] - Notify the parent how favourable it would be for this node to run.
- [`RunTimer`][run-timer] - Time since an action started/stopped.


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