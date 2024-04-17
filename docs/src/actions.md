# Actions

Beet has a growing list of actions. For now the best place to look for examples is the tests at the bottom of the file for each action.

- [Graph Roles](./concepts.md#graph-roles) are a way of categorizing actions.

## `EcsModule`

The [`EcsModule`][EcsModule] contains the basic actions and components required for most behaviors.

| Name                          | Graph Role | Description                                                      |
| ----------------------------- | ---------- | ---------------------------------------------------------------- |
| `EmptyAction`                 | Node       | Does what it says on the tin, useful for tests                   |
| `Repeat`                      | Node       | Reattaches the `Running` component whenever it is removed.       |
| `InsertInDuration<RunResult>` | Node       | Adds a `RunResult` after a given duration.                       |
| `SetOnSpawn<Score>`           | Node       | Sets the score to a constant value when this behavior is spawned |
| `InsertOnRun<RunResult>`      | Node       | Immediately succeed or fail when this behavior runs              |
| `SequenceSelector`            | Child      | Run children in sequence until one fails                         |
| `FallbackSelector`            | Child      | Run children in sequence until one succeeds                      |
| `ScoreSelector`               | Child      | Run the child with the highest score                             |

## `CoreModule`

The [`CoreModule`][CoreModule] contains more domain-specific actions, ie movement.

| Name                      | Graph Role | Description                                                          |
| ------------------------- | ---------- | -------------------------------------------------------------------- |
| `Hover`                   | Agent      | Translate the agent up and down in a sine wave                       |
| `Translate`               | Agent      | Apply constant translation                                           |
| `SetAgentOnRun<Velocity>` | Agent      | Set the `Velocity` of an agent on run                                |
| `Seek`                    | Agent      | Go to the agent's `SteerTarget` with an optional `ArriveRadius`      |
| `Wander`                  | Agent      | Somewhat cohesive random walk                                        |
| `SucceedOnArrive`         | Agent      | Succeeds when the agent arrives at the `SteerTarget`                 |
| `FindSteerTarget`         | Agent      | Sets the `SteerTarget` when an entity with the given name is nearby. |
| `ScoreSteerTarget`        | Node       | Adjusts the `Score` based on distance to the `SteerTarget`           |
| `DespawnSteerTarget`      | World      | Immediately recursively despawns the `SteerTarget`                   |

## `MlModule`

The [`MlModule`][MlModule] contains actions that use machine learning.

| Name             | Graph Role | Description                                                                                                              |
| ---------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------ |
| `SentenceScorer` | Child      | Updates the `Score` of each child based on the similarity of its `Sentence` with the agent, for use with `ScoreSelector` |


[EcsModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/ecs_module.rs
[CoreModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/core_module/core_module.rs
[MlModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ml/src/ml_module/ml_module.rs