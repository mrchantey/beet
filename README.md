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
use bevy::prelude::*;
use beet::prelude::*;

fn main(){

  let mut app = App::new();

  app.add_plugins((
    DefaultPlugins,
    DefaultBeetPlugins
  ));

  app.world_mut().spawn((
      Running,
      Repeat,
      SequenceSelector::default(), 
    ))
    .with_children(|parent| {
      parent.spawn((
        LogOnRun("Hello".into()),
        InsertOnRun(RunResult::Success),
      ));
      parent.spawn((
        LogOnRun("World".into()),
        InsertOnRun(RunResult::Success),
      ));
    });

  app.run();

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