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
// A demonstration of Sequence control flow
world.spawn(SequenceFlow)
	.with_child((
		Name::new("Hello"),
		EndOnRun::success(),
	))
	.with_child((
		Name::new("World"),
		EndOnRun::success(),
	))
	.trigger(OnRun);
```

[bevy-observers]:https://docs.rs/bevy/latest/bevy/ecs/observer/struct.Observer.html#


## Examples

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
