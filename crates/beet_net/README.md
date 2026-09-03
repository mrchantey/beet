# `beet_net`

Transport agnostic networking for bevy applications.

The `Request` / `Response` pattern is generalized and not tied to any transport, with implementations in `http` and `stdio`. See [this blog post](https://beet.org/blog/its-all-been-done-before) for more about the agnostic philosophy.

## Features

- **Transport agnostic servers**: server implementations for cli arguments or http requests
- **Cross-platform clients**: HTTP clients for sending requests (ureq, reqwest and WASM backends)
- **Object storage**: Store-based storage abstraction (filesystem, S3, etc.)
- **Sockets**: WebSocket client and server
- **Action-based exchanges**: Request/response handling via `Action<Request, Response>` from `beet_action`

## Servers and lifecycle verbs

A server (`HttpServer`, `CliServer`, `TuiServer`, `ReplServer`, ...) is a long-running *facet* on its entity's `RunningSet` (see `beet_action`), and the dispatch host is its **child**: one server reads as `<HttpServer><Router>..</Router></HttpServer>`, several as `<CallOnReady {(A, B)}><Router>..</Router></CallOnReady>`. `RunningSetFilter` owns the `--server` grammar every server's `select` closure reads. `exchange()` calls *this* entity's `Request -> Response` action; `exchange_child()` is the downward hop a server uses to reach the first child serving that pair.

Two verbs bind the entity lifecycle to those actions, one per edge:

- [`CallOnReady`] is the load verb: on the entity's `Ready` it calls the entity's action with the process request and streams/exits, trying `Request -> Response`, then `() -> Outcome`, then `() -> ()`. It fires on every load, so a file says exactly what happens when it is loaded; a loader building a document to render rather than run disarms the subtree with [`DisableCallOnReady`], and an explicit `CallOnReady::call` ignores the disarm. There is no wrapper command that loads one entry into another's process: an entry that is its own CLI is launched directly and names its verb on argv (`beet --main=site serve --server=http`), the identical path the deployed unit's `ExecStart` takes.
- [`CallOnStart`] is the start verb, the same shape on the other edge: a run root declaring `{SweepDescendants}` sweeps `StartRunning<Request>` over its subtree, and `CallOnStart` observes its own entity, calling its action detached. An undeclared start fires on its entity alone, deliberately: actions don't magically run, and the sweep never leaves its root's subtree, so co-resident entries never start each other's work.

## Stores and the repo store

A `BlobStore` is the erased handle every backend (`FsStore`, `S3Store`, `InMemoryStore`, the browser stores) materializes on its entity, so a consumer never names a backend. Exactly one store in an app is the **repo store**: the canonical store an entry loads through, marked `RepoStore` and rooted on the entry root, holding the entry document, its templates, routes and assets. A relative path anywhere in the app means a path in it, and a second one anywhere in the world is an error.

Consumers usually resolve it by ancestry (the `AncestorQuery<&BlobStore>` idiom, which honours any intervening `DirPath` scope); `RepoStore::get` is the direct lookup. Every other store is a plain store, declared for a purpose and reached by name through a `StoreRef`. See `src/store/mod.rs` for the backends and the reactive `BlobEvent` substrate.

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
      exchange_handler(|_| {
        Response::ok_body("hello world", MediaType::Text)
      }),
    ));
  })
  .run();
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `server` | Lightweight mini HTTP server using `async-io` TCP |
| `hyper` | Full-featured Hyper HTTP server (implies `server`) |
| `lambda` | AWS Lambda server support |
| `aws_sdk` | AWS S3 and DynamoDB providers |
| `reqwest` | Use reqwest as the HTTP client backend |
| `ureq` | Use ureq as the HTTP client backend |
| `tungstenite` | Native WebSocket support |
| `russh_client` / `russh_server` | SSH client and server |
| `webdriver` | WebDriver browser automation client |
| `mdns` / `udp` | mDNS service discovery and UDP sockets |
| `rustls-tls` | Use rustls for TLS |
| `native-tls` | Use native TLS implementation |
| `secure` | TLS serving: the `Tls` component (self-signed dev cert or provided PEM files) |