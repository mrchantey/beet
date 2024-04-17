# Actions

Beet has a growing list of actions. For now the best place to look for examples is the tests at the bottom of the file for each action.

### Graph Roles

Actions can be categorized by what part of the world they mutate:

## `EcsModule`

The [`EcsModule`][EcsModule] contains the core actions and components required for most behaviors.

| Name                                   | [Graph Role](./concepts.md#graph-roles) | Description                                                    |
| -------------------------------------- | --------------------------------------- | -------------------------------------------------------------- |
| `SetOnStart<Score>`                    | Node                                    | Sets the score to a constant value when this entity is spawned |
| [`SequenceSelector`][SequenceSelector] | Child                                   | Run children in sequence until one fails                       |
| [`FallbackSelector`][FallbackSelector] | Child                                   | Run children in sequence until one succeeds                    |
| [`ScoreSelector`][ScoreSelector]       | Child                                   | Run the child with the highest score                           |


[SequenceSelector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/sequence_selector.rs
[FallbackSelector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/fallback_selector.rs
[ScoreSelector]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/selectors/score_selector.rs
[EcsModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/ecs_module.rs