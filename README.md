# beet

<div align="center">
  <p>
    <strong>A very flexible behavior library for games and robotics.</strong>
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

```rust
// A demonstration of Sequence control flow
fn main() {
	App::new()
		.add_plugins((
			LogPlugin::default(), 
			BeetObserverPlugin
		))
		.world_mut()
		.spawn((
			Name::new("root"), 
			LogNameOnRun, 
			SequenceFlow
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("child1"),
				LogNameOnRun,
				EndOnRun::success(),
			));
			parent.spawn((
				Name::new("child2"),
				LogNameOnRun,
				EndOnRun::success(),
			));
		})
		.flush_trigger(OnRun);

// Running: root
// Running: child1
// Running: child2
}
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