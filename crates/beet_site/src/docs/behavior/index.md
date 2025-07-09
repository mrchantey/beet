+++
title = "Behavior"
description= "Use bevy scenes to craft"
+++

Define behaviors using regular entities and relations with `beet_flow`:

```rust title="hello_fallback.rs"
world.spawn((
  RunOnSpawn
  FallbackFlow,
  children![
    (
      LogOnRun::("Hello"),
      EndOnRun::failure(),
    ),
    (
      LogOnRun::("World"),
      EndOnRun::success(),
    )
  ]
));
```
