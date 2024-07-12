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

Beet is Behavior Expressed as Entity Trees, using [bevy observers][bevy-observers] for control flow and messaging. The entity-based approach is very flexible, and allows for multiple behavior paradigms to be used together as needed.

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
## Examples

Most examples rely on assets that can be downloaded with the following commands, or manually from [here](https://storage.googleapis.com/beet-misc/assets.tar.gz).

```sh
curl -o ./assets.tar.gz https://storage.googleapis.com/beet-misc/assets.tar.gz
tar -xzvf ./assets.tar.gz
rm ./assets.tar.gz
```

The examples for beet are *scene-based*. As Bevy scene workflows are a wip, there are a few `Placeholder` types used for not-yet-serializable types like cameras, asset handles etc.

## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |


[bevy-observers]:(https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html#)