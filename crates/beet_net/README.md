# `beet_net`

Very bevy networking utilities.

This crate provides cross-platform networking and communication primitives for bevy applications.

## Modules

- **client**: HTTP client for sending requests (ureq, reqwest and WASM backends)
- **exchange**: Request/response exchange patterns for Bevy entities
- **object_storage**: Bucket-based storage abstraction (filesystem, S3, etc.)
- **server**: HTTP server implementations
- **sockets**: WebSocket client and server

## Example

```rust,ignore
use beet_net::prelude::*;
use beet_core::prelude::*;

// Create a simple server with a handler
App::new()
    .add_plugins((MinimalPlugins, ServerPlugin))
    .add_systems(Startup, |mut commands: Commands| {
        commands.spawn((
            HttpServer::default(),
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
