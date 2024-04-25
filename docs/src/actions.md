# Actions

Beet has a growing list of actions. They are separated into modules for ease of use.
For now the best place to find usage examples is the tests at the bottom of the file for each action.


## Lifecycle Actions

Often we want to do something when a behavior is spawned or starts running. [Lifecycle actions][lifecycle-actions] are generic and have a range of use cases.

- `InsertOnRun<T>` - Inserts a component when this behavior starts running
- `SetOnRun<T>` - Sets a component when this behavior starts running
- `SetAgentOnRun<T>` - Sets an agent's component when this behavior starts running
- `SetOnSpawn<T>` - Sets a component when this behavior spawns

## `EcsModule`

Basic actions required for lifecycle handling and debugging.

*[Graph Roles](./concepts.md#graph-roles) are a way of categorizing actions.*

| Name                                      | Graph Role | Description                                                                                |
| ----------------------------------------- | ---------- | ------------------------------------------------------------------------------------------ |
| [<h3>EcsModule</h3>](EcsModule)           |            |                                                                                            |
| `InsertInDuration<RunResult>`             | Node       | Adds a `RunResult` after a given duration.                                                 |
| `InsertOnRun<RunResult>`                  | Node       | Immediately succeed or fail when this behavior runs                                        |
| `LogNameOnRun`                            | Node       | Logs the `Name` when the action is run.                                                    |
| `LogOnRun`                                | Node       | Logs a message when the action is run.                                                     |
| `Repeat`                                  | Node       | Reattaches the `Running` component whenever it is removed.                                 |
| `SetOnSpawn<Score>`                       | Node       | Sets the score to a constant value when this behavior is spawned                           |
| `EmptyAction`                             | Node       | Does what it says on the tin, useful for tests                                             |
| `FallbackSelector`                        | Child      | Run children in sequence until one succeeds                                                |
| `SequenceSelector`                        | Child      | Run children in sequence until one fails                                                   |
| `ParallelSelector`                        | Child      | Run children in parallel until one finishes                                                |
| `ScoreSelector`                           | Child      | Run the child with the highest score                                                       |
| [<h3>MovementModule</h3>](MovementModule) |            |                                                                                            |
| `Hover`                                   | Agent      | Translates the agent up and down in a sine wave                                            |
| `Translate`                               | Agent      | Applies constant translation                                                               |
| `SetAgentOnRun<Velocity>`                 | Agent      | Sets the `Velocity` of an agent on run                                                     |
| [<h3>SteerModule</h3>](SteerModule)       |            |                                                                                            |
| `Seek`                                    | Agent      | Go to the agent's `SteerTarget` with an optional `ArriveRadius`                            |
| `Wander`                                  | Agent      | Somewhat cohesive random walk                                                              |
| `SucceedOnArrive`                         | Agent      | Succeeds when the agent arrives at the `SteerTarget`                                       |
| `FindSteerTarget`                         | Agent      | Sets the `SteerTarget` when an entity with the given name is nearby.                       |
| `ScoreSteerTarget`                        | Node       | Adjusts the `Score` based on distance to the `SteerTarget`                                 |
| `DespawnSteerTarget`                      | World      | Recursively despawns the `SteerTarget`                                                     |
| [<h3>RoboticsModule</h3>](RoboticsModule) |            |                                                                                            |
| `SetAgentOnRun<DualMotorValue>`           | Agent      | Sets the `DualMotorValue` of an agent on run                                               |
| `DepthSensorScorer`                       | Node       | Sets the [`Score`] based on the [`DepthSensor`] value                                      |
| [<h3>`MlModule`</h3>](MlModule)           |            |                                                                                            |
| `SentenceScorer`                          | Child      | Updates the `Score` of each child based on the similarity of its `Sentence` with the agent |

[lifecycle-actions]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/actions/lifecycle_actions.rs

[EcsModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/ecs_module.rs
[CoreModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/core_module/core_module.rs
[MovementModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/movement/movement_module.rs
[MlModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ml/src/ml_module/ml_module.rs

