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

A server (`HttpServer`, `CliServer`, `TuiServer`, `DomServer`, `ReplServer`, ...) is a long-running *facet* on its entity's `RunningSet` (see `beet_action`), and the dispatch host is its **child**: one server reads as `<HttpServer><Router>..</Router></HttpServer>`, several as `<CallOnReady {(A, B)}><Router>..</Router></CallOnReady>`. `RunningSetFilter` owns the `--server` grammar every server's `select` closure reads. A surface server (`TuiServer` on a terminal, `DomServer` on a browser tab's document) spawns a page host with an in-world `Navigator` at the request path rather than a listener, so the same entry paints wherever it is launched. `exchange()` calls *this* entity's `Request -> Response` action; `exchange_child()` is the downward hop a server uses to reach the first child serving that pair.

Two verbs bind the entity lifecycle to those actions, one per edge:

- [`CallOnReady`] is the load verb: on the entity's `Ready` it calls the entity's action with the process request and streams/exits, trying `Request -> Response`, then `() -> Outcome`, then `() -> ()`. It fires on every load, so a file says exactly what happens when it is loaded; a loader building a document to render rather than run disarms the subtree with [`DisableCallOnReady`], and an explicit `CallOnReady::call` ignores the disarm. There is no wrapper command that loads one entry into another's process: an entry that is its own CLI is launched directly and names its verb on argv (`beet --main=site serve --server=http`), the identical path the deployed unit's `ExecStart` takes.
- [`CallOnStart`] is the start verb, the same shape on the other edge: a run root declaring `{SweepDescendants}` sweeps `StartRunning<Request>` over its subtree, and `CallOnStart` observes its own entity, calling its action detached. An undeclared start fires on its entity alone, deliberately: actions don't magically run, and the sweep never leaves its root's subtree, so co-resident entries never start each other's work.

**What a client may make a server allocate is declared, not inherited from the traffic.** `ServerLimits` is `#[require]`d by `HttpServer` and read once per accept loop, so every socket-backed server (mini and hyper) answers under a cap: `max_in_flight` permits taken around the *dispatch* (not the connection, so a slow client holds nothing back), `max_body_bytes` refusing an oversized `content-length` with a `413` before a byte of the body is read or allocated, and `dispatch_timeout` answering `503` rather than hanging. Override per server, `<HttpServer {ServerLimits{max_in_flight:4}}/>`. These are remote-OOM guards, not tuning knobs: both halves were live defects that killed `beet.org` (2026-09-30), an unbounded accept loop spending ~13 MB per in-flight request for the dearest route and a body read sized from the client's own header. Lower `max_in_flight` is *faster* as well as lighter, since a page builds on the one world thread and concurrency past a handful buys queueing rather than throughput. The one hazard it creates is a handler that calls its own server (the charcell image fetch reaches the canonical loopback port like any client): it holds a permit while waiting for a second, which is what `dispatch_timeout` exists to break. A loopback exemption is not the answer — a deployed box is reverse-proxied from `localhost`, so every real request is loopback.

## Stores and the repo store

A `BlobStore` is the erased handle every backend (`FsStore`, `S3Store`, `InMemoryStore`, `SqliteStore`, the browser stores, the read-only `HttpStore` over a served `<ServeBlobs>` mount) materializes on its entity, so a consumer never names a backend. A backend is selected by a `StoreUri` (`fs:<path>`, `s3://<bucket>`, `dynamo://<table>`, `sqlite:<path>`, `indexed-db://<db>`, `http:<prefix>`, each carrying its own root) and built by `StoreProvider::from_uri`, the one place a uri becomes a concrete store component; `StoreProvider::compose` forks any of them into a `StoreFork` (a local store reads try first and writes land in), the fork a process keeps of a repo it reads but does not own, the store-grain twin of `SceneFork`; in a browser the fork's first write sets one local-storage bit (`StoreUri::fork_mark`) a served page reads before its world exists, to hide the published page from a returning editor. Exactly one store in an app is the **repo store**: the canonical store an entry loads through, marked `RepoStore` and rooted on the entry root, holding the entry document, its templates, routes and assets. A relative path anywhere in the app means a path in it, and a second one anywhere in the world is an error.

Consumers usually resolve it by ancestry (the `AncestorQuery<&BlobStore>` idiom, which honours any intervening `DirPath` scope); `RepoStore::get` is the direct lookup. Every other store is a plain store, declared for a purpose and reached by name through a `StoreRef`. A consumer of several stores names them one field per role in one place instead — its own `#[field(required)] #[entities]` fields where it is an action (`AnalyticsRollupJob{raw, rollups, archive}`), one shared component where several consumers need the same set — since a generic `StoreRef` beside a specifically named twin leaves the generic one silently role-bearing. See `src/store/mod.rs` for the backends and the reactive `BlobEvent` substrate.

`src/store_actions/` is what markup and an agent toolset reach those stores through: `ListBlobs`/`ReadBlob`/`WriteBlob`/`EditText`/`RemoveBlob` over the nearest ancestor `BlobStore`, and `SqlSelect`, one authored `SELECT` over the `SqliteStore` a `StoreRef` names (`<Route path="senders" {(SqlSelect{sql:".."}, StoreRef($index))}/>`). There is no write twin of `SqlSelect` on purpose: a SQLite store a consumer queries is an index, whose schema comes from whatever fills it and whose content is rebuilt from that source.

`SqliteStore` runs on every target, browsers included: `rusqlite` builds its ffi from `sqlite-wasm-rs` rather than `libsqlite3-sys` on `wasm32-unknown-unknown`, so the SQL dialect, the generated columns and the query planner are the same engine everywhere. Two things differ there and both are the default VFS holding the database in memory rather than in a file: the store exists while a connection to it does (there is nothing to stat), and there is no WAL. Nor is there a thread to hand a query to, so a call runs where it is made — which is why a browser build that cares about jank, or wants a durable OPFS VFS, belongs in a dedicated worker.

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
| `webdriver` | WebDriver browser automation client: BiDi sessions, auto-waiting finds, trusted input (a secret typed without a trace), console/network collectors, cookies, persistent profiles, screenshots and PDF export |
| `ooxml` | Office Open XML over `ooxmlsdk`: a Word file's tables, checkboxes and runs and a workbook's unlocked cells read and filled through a `Blob`, Word files and slide decks transcoded to HTML (which `beet_ui`'s `OoxmlParser` parses as media), and the `<OoxmlCells/>` dump; see `src/ooxml/mod.rs` |
| `mdns` / `udp` | mDNS service discovery and UDP sockets |
| `rustls-tls` | Use rustls for TLS |
| `native-tls` | Use native TLS implementation |
| `secure` | TLS serving: the `Tls` component (self-signed dev cert or provided PEM files) |
| `atproto` | An account's repo on a real PDS: `XrpcPds`, the `AtprotoAuth` seam with `AppPassword`, did and handle resolution, and the `app.bsky.*` lexicons beet writes (`FeedPost`, `RichText`). The `Pds` family itself (`EmulatorPds`, the converge, `<AtprotoAccount/>`) rides `json`; see `src/atproto/mod.rs` |