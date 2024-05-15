# Actions

## Action List

| Name                                          | Category [?](./concepts.md#action-category) | Description                                                                                               |
| --------------------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| <h3>Generic Lifecycle Actions</h3>            |                                             |                                                                                                           |
| `InsertOnRun<T>`                              | Behavior                                    | Inserts a component when this behavior starts running                                                     |
| `SetOnRun<T>`                                 | Behavior                                    | Sets a component when this behavior starts running                                                        |
| `SetOnSpawn<T>`                               | Behavior                                    | Sets a component when this behavior spawns                                                                |
| `SetAgentOnRun<T>`                            | Agent                                       | Sets an agent's component when this behavior starts running                                               |
| <h3>[LifecyclePlugin][LifecyclePlugin]</h3>   |                                             |                                                                                                           |
| `InsertInDuration<RunResult>`                 | Behavior                                    | Adds a `RunResult` after a given duration.                                                                |
| `InsertOnRun<RunResult>`                      | Behavior                                    | Immediately succeed or fail when this behavior runs                                                       |
| `LogOnRun`                                    | Behavior                                    | Logs a message when the action is run.                                                                    |
| `Repeat`                                      | Behavior                                    | Reattaches the `Running` component whenever it is removed.                                                |
| `SetOnSpawn<Score>`                           | Behavior                                    | Sets the score to a constant value when this behavior is spawned                                          |
| `EmptyAction`                                 | Behavior                                    | Does what it says on the tin, useful for tests                                                            |
| `FallbackSelector`                            | ChildBehaviors                              | Run children in sequence until one succeeds                                                               |
| `ParallelSelector`                            | ChildBehaviors                              | Run children in parallel until one finishes                                                               |
| `SequenceSelector`                            | ChildBehaviors                              | Run children in sequence until one fails                                                                  |
| `ScoreSelector`                               | ChildBehaviors                              | Run the child with the highest score                                                                      |
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
| `ScoreSteerTarget`                            | Behavior                                    | Adjusts the `Score` based on distance to the `SteerTarget`                                                |
| `DespawnSteerTarget`                          | World                                       | Recursively despawns the `SteerTarget`                                                                    |
| <h3>[`AnimationPlugin`][AnimationPlugin]</h3> |                                             |                                                                                                           |
| `PlayAnimation`                               | Agent                                       | Play an animation on the agent when this action starts running.                                           |
| `InsertOnAnimationEnd<RunResult>`             | Agent                                       | Inserts the given `RunResult` when an animation is almost finished.                                       |
| <h3>[`RoboticsPlugin`][RoboticsPlugin]</h3>   |                                             |                                                                                                           |
| `SetAgentOnRun<DualMotorValue>`               | Agent                                       | Sets the `DualMotorValue` of an agent on run                                                              |
| `DepthSensorScorer`                           | Behavior                                    | Sets the [`Score`] based on the [`DepthSensor`] value                                                     |
| <h3>[`MlPlugin`][MlPlugin]</h3>               |                                             |                                                                                                           |
| `SentenceScorer`                              | ChildBehaviors                              | Updates the `Score` of each child based on the similarity of its `Sentence` with the agent's              |
| `FindSentenceSteerTarget`                     | Agent                                       | Finds the `Sentence` with the highest similarity to the agent's, then set it as the agent's steer target. |
| <h3>`UI`</h3>                                 |                                             |                                                                                                           |
| `SetTextOnRun`                                | World                                       | Sets the `Text` of all entities matching the query on run.                                                |
|                                               |


[LifecyclePlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/lifecycle/lifecycle_plugin.rs
[MovementPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/movement/movement_plugin.rs
[SteerPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/steer/steer_plugin.rs
[AnimationPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/animation/animation_plugin.rs
[MlPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_ml/src/ml_module/ml_plugin.rs
[RoboticsPlugin]:https://github.com/mrchantey/beet/blob/main/crates/beet_core/src/robotics/robotics_plugin.rs