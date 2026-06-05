+++
title = "beet_net"
+++

# beet_net

`beet_net` generalises the `Request`/`Response` pattern and cuts it loose from any single transport. A request has a path, params and a body; a response has a status code and a body. Nothing in that shape mentions HTTP, and that is deliberate.

Once an exchange is just data, the transport becomes a swappable detail. The same handler can sit behind a `CliServer` reading command-line arguments or an `HttpServer` taking HTTP requests, with the only change being which server you spawn.

```rust,ignore
use beet_net::prelude::*;
use beet_core::prelude::*;

App::new()
  .add_plugins((MinimalPlugins, ServerPlugin))
  .add_systems(Startup, |mut commands: Commands| {
    commands.spawn((
      // swap CliServer for HttpServer to take HTTP requests instead
      CliServer::default(),
      exchange_handler(|_| {
        Response::ok_body("hello world", MediaType::Text)
      }),
    ));
  })
  .run();
```

This is what lets a beet app's `--help` flag, its HTTP sitemap and its AI tool definitions be the same thing rather than three parallel implementations. Around that core, beet_net provides cross-platform HTTP clients (ureq, reqwest and a wasm backend), object storage over filesystems or S3, WebSockets, and SSH, each behind a feature flag so the dependency footprint stays small. Handlers themselves are `Action<Request, Response>` values from [beet_action](/docs/crates/beet_action), which is how routing in [beet_router](/docs/crates/beet_router) builds on top.
