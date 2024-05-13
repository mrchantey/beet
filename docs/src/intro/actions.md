# Actions

Beet has a growing list of built-in actions, they can be combined with eachother and custom actions to create all kinds of behaviors.

## Generic Lifecycle Actions

Often we want to insert or change a component when a behavior is spawned or starts running.

- `InsertOnRun<T>` - Inserts a component when this behavior starts running
- `SetOnRun<T>` - Sets a component when this behavior starts running
- `SetAgentOnRun<T>` - Sets an agent's component when this behavior starts running
- `SetOnSpawn<T>` - Sets a component when this behavior spawns

## Action List

| Name                                          | Category [?](./concepts.md#action-category) | Description                                                                                               |
| --------------------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| <h3>[LifecyclePlugin][LifecyclePlugin]</h3>   |                                             |                                                                                                           |
| `InsertInDuration<RunResult>`                 | Internal                                    | Adds a `RunResult` after a given duration.                                                                |
| `InsertOnRun<RunResult>`                      | Internal                                    | Immediately succeed or fail when this behavior runs                                                       |
| `LogOnRun`                                    | Internal                                    | Logs a message when the action is run.                                                                    |
| `Repeat`                                      | Internal                                    | Reattaches the `Running` component whenever it is removed.                                                |
| `SetOnSpawn<Score>`                           | Internal                                    | Sets the score to a constant value when this behavior is spawned                                          |
| `EmptyAction`                                 | Internal                                    | Does what it says on the tin, useful for tests                                                            |
| `FallbackSelector`                            | Children                                    | Run children in sequence until one succeeds                                                               |
| `ParallelSelector`                            | Children                                    | Run children in parallel until one finishes                                                               |
| `SequenceSelector`                            | Children                                    | Run children in sequence until one fails                                                                  |
| `ScoreSelector`                               | Children                                    | Run the child with the highest score                                                                      |
| <h3>[`MovementPlugin`][MovementPlugin]</h3>   |                                             |                                                                                                           |
| `Hover`                                       | Agent                                       | Translates the agent up and down in a sine wave                                                           |
| `Translate`                                   | Agent                                       | Applies constant translation                                                                              |
| `SetAgentOnRun<Velocity>`                     | Agent                                       | Sets the `Velocity` of an agent on run                                                                    |
| <h3>[`SteerPlugin`][SteerPlugin]</h3>         |                                             |                                                                                                           |
| `Seek`                                        | Agent                                       | Go to the agent's `SteerTarget` with an optional `ArriveRadius`                                           |
| `Wander`                                      | Agent                                       | Somewhat cohesive random walk                                                                             |
| `Separate::<GroupSteerAgent>`                 | Agent                                       | Separate from entities with `GroupSteerAgent`.                                                            |
| `Align::<GroupSteerAgent>`                    | Agent                                       | Align `Velocity` with that of entities with `GroupSteerAgent`.                                            |
| `Cohere::<GroupSteerAgent>`                   | Agent                                       | Move towards the center of mass of entities with `GroupSteerAgent`.                                       |
| `SucceedOnArrive`                             | Agent                                       | Succeeds when the agent arrives at the `SteerTarget`                                                      |
| `FindSteerTarget`                             | Agent                                       | Sets the `SteerTarget` when an entity with the given name is nearby.                                      |
| `ScoreSteerTarget`                            | Internal                                    | Adjusts the `Score` based on distance to the `SteerTarget`                                                |
| `DespawnSteerTarget`                          | World                                       | Recursively despawns the `SteerTarget`                                                                    |
| <h3>[`AnimationPlugin`][AnimationPlugin]</h3> |                                             |                                                                                                           |
| `PlayAnimation`                               | Agent                                       | Play an animation on the agent when this action starts running.                                           |
| `InsertOnAnimationEnd<RunResult>`             | Agent                                       | Inserts the given `RunResult` when an animation is almost finished.                                       |
| <h3>[`RoboticsPlugin`][RoboticsPlugin]</h3>   |                                             |                                                                                                           |
| `SetAgentOnRun<DualMotorValue>`               | Agent                                       | Sets the `DualMotorValue` of an agent on run                                                              |
| `DepthSensorScorer`                           | Internal                                    | Sets the [`Score`] based on the [`DepthSensor`] value                                                     |
| <h3>[`MlPlugin`][MlPlugin]</h3>               |                                             |                                                                                                           |
| `SentenceScorer`                              | Children                                    | Updates the `Score` of each child based on the similarity of its `Sentence` with the agent's              |
| `FindSentenceSteerTarget`                     | Agent                                       | Finds the `Sentence` with the highest similarity to the agent's, then set it as the agent's steer target. |

[LifecyclePlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/lifecycle/lifecycle_plugin.rs
[MovementPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/movement/movement_plugin.rs
[SteerPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steer/steer_plugin.rs
[AnimationPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/animation/animation_plugin.rs
[MlPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_ml/src/ml_module/ml_plugin.rs
[RoboticsPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/robotics/robotics_plugin.rs