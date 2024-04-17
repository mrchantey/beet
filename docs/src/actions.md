# Actions

Beet has a growing list of actions. For now the best place to look for examples is the tests at the bottom of the file for each action.

[Graph Roles](./concepts.md#graph-roles) are a way of categorizing actions.

## `EcsModule`

The [`EcsModule`][EcsModule] contains the core actions and components required for most behaviors.

| Name                     | Graph Role | Description                                                      |
| ------------------------ | ---------- | ---------------------------------------------------------------- |
| `EmptyAction`            | Node       | Does what it says on the tin, useful for tests                   |
| `Repeat`                 | Node       | Reattaches the `Running` component whenever it is removed.       |
| `SetOnStart<Score>`      | Node       | Sets the score to a constant value when this behavior is spawned |
| `InsertOnRun<RunResult>` | Node       | Immediately succeed or fail when this behavior runs              |
| `SequenceSelector`       | Child      | Run children in sequence until one fails                         |
| `FallbackSelector`       | Child      | Run children in sequence until one succeeds                      |
| `ScoreSelector`          | Child      | Run the child with the highest score                             |

## `CoreModule`

[EcsModule]:https://github.com/mrchantey/beet/blob/main/crates/beet_ecs/src/ecs_module/ecs_module.rs