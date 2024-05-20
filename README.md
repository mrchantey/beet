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
    <a href="https://mrchantey.github.io/beet">Book</a>
    <span> | </span>
    <a href="https://docs.rs/beet">API Docs</a>
    <!-- <span> | </span>
    <a href="https://mrchantey.github.io/beet/other/contributing.html">Contributing</a> -->
  </h3>

  <sub>made with ❤️‍🔥 by <a href="https://github.com/mrchantey">mrchantey</a></sub>
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

## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.14.0 | 0.0.2  |
