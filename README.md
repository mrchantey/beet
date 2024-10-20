# beet

<div align="center">
  <p>
    <strong>A modular behavior library for the Bevy Engine.</strong>
  </p>
  <p>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/v/beet.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/d/beet.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/beet"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
  <h3>
    <a href="https://beetmash.com/docs/beet">Guidebook</a>
    <span> | </span>
    <a href="https://docs.rs/beet">API Docs</a>
    <!-- <span> | </span>
    <a href="https://mrchantey.github.io/beet/other/contributing.html">Contributing</a> -->
  </h3>
</div>

Beet is behavior expressed as entity trees, using [Observers][bevy-observers] for control flow. The entity-based approach is very flexible and allows for multiple behavior paradigms to be used together as needed.

Currently implemented paradigms:
- [Behavior Trees](./examples/hello_world.rs)
- [Basic Utility AI](./examples/hello_utility_ai.rs)
- [LLM Sentence Similarity](./examples/hello_ml.rs)
- [Reinforcement Learning](./examples/frozen_lake_train.rs)


## Hello World

```rust
// A demonstration of Fallback control flow
world.spawn(FallbackFlow)
  .with_children(|parent| {
    parent.spawn((
      LogOnRun::("Hello"),
      EndOnRun::failure(),
    ));
    parent.spawn((
      LogOnRun::("World"),
      EndOnRun::success(),
    ));
  })
```


[bevy-observers]:https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html#


## Examples


> ⚠️⚠️⚠️ If you'd like to check out this repo please use the stable [v0.0.3](https://github.com/mrchantey/beet/tree/v0.0.3) commit ⚠️⚠️⚠️ 
>
> Beet and my other crates it depends on are currently on a scene serialization  bugfix Bevy fork, see [this issue](https://github.com/bevyengine/bevy/issues/14300) for details. The fix is scheduled for the `0.14.2` milestone so fingers crossed we'll be back on bevy main from then.



The examples for beet are *scene-based*, meaning each example provides a scene for a common base app. As Bevy scene workflows are a wip, there are a few `Placeholder` types used for not-yet-serializable types like cameras, asset handles etc.

Most examples rely on assets that can be downloaded with the following commands, or manually from [here](https://beetmash-public.s3.us-west-2.amazonaws.com/assets.tar.gz).

```sh
curl -o ./assets.tar.gz https://beetmash-public.s3.us-west-2.amazonaws.com/assets.tar.gz
tar -xzvf ./assets.tar.gz
rm ./assets.tar.gz
```


## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |

## TODO

- When we get [`OnMutate`](https://github.com/bevyengine/bevy/pull/14520) observers, they should probably replace most `OnInsert` observers we're using
