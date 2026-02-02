# `beet_net`

Transport agnostic networking for bevy applications.

The `Request` / `Response` pattern is generalized and not tied to any transport, with implementations in `http` and `stdio`. See [this blog post](https://beetstack.dev/blog/post-8) for more about the agnostic philosophy.

## Features

- **Transport agnostic servers**: server implementations for cli arguments or http requests
- **Cross-plaform clients**: HTTP clients for sending requests (ureq, reqwest and WASM backends)
- **Object_storage**: Bucket-based storage abstraction (filesystem, S3, etc.)
- **Sockets**: WebSocket client and server

## Example

```rust,ignore
use beet_net::prelude::*;
use beet_core::prelude::*;

// Create a simple server with a handler
App::new()
  .add_plugins((MinimalPlugins, ServerPlugin))
  .add_systems(Startup, |mut commands: Commands| {
    commands.spawn((
			// swap out the server to handle http requests!
			CliServer::default(),
      // HttpServer::default(),
      handler_exchange(|_, _| {
        Response::ok_body("hello world", "text/plain")
      }),
    ));
  })
  .run();
```

## Features

| Feature | Description |
|---------|-------------|
| `server` | HTTP server functionality |
| `lambda` | AWS Lambda server support |
| `aws` | AWS S3 and DynamoDB providers |
| `flow` | `beet_flow` integration for exchange patterns |
| `reqwest` | Use reqwest as the HTTP client backend |
| `ureq` | Use ureq as the HTTP client backend |
| `tungstenite` | Native WebSocket support |
| `rustls-tls` | Use rustls for TLS |
| `native-tls` | Use native TLS implementation |
