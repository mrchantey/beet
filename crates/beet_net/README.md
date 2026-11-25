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

A server (`HttpServer`, `CliServer`, `TuiServer`, `DomServer`, `ReplServer`, ...) is a long-running *facet* on its entity's `RunningSet` (see `beet_action`), and the dispatch host is its **child**: one server reads as `<HttpServer><Router>..</Router></HttpServer>`, several as `<CallOnReady {(A, B)}><Router>..</Router></CallOnReady>`. `ServerFilter` owns the `--server` grammar every server's `select` closure reads. A surface server (`TuiServer` on a terminal, `DomServer` on a browser tab's document) spawns a page host with an in-world `Navigator` at the request path rather than a listener, so the same entry paints wherever it is launched. `exchange()` calls *this* entity's `Request -> Response` action; `exchange_child()` is the downward hop a server uses to reach the first child serving that pair.

Two verbs bind the entity lifecycle to those actions, one per edge:

- [`CallOnReady`] is the load verb: on the entity's `Ready` it calls the entity's action with the process request and streams/exits, trying `Request -> Response`, then `() -> Outcome`, then `() -> ()`. It fires on every load, so a file says exactly what happens when it is loaded; a loader building a document to render rather than run disarms the subtree with [`DisableCallOnReady`], and an explicit `CallOnReady::call` ignores the disarm. There is no wrapper command that loads one entry into another's process: an entry that is its own CLI is launched directly and names its verb on argv (`beet --entry=site serve --server=http`), the identical path the deployed unit's `ExecStart` takes.
- [`CallOnStart`] is the start verb, the same shape on the other edge: a run root declaring `{SweepDescendants}` sweeps `StartRunning<Request>` over its subtree, and `CallOnStart` observes its own entity, calling its action detached. An undeclared start fires on its entity alone, deliberately: actions don't magically run, and the sweep never leaves its root's subtree, so co-resident entries never start each other's work.

**What a client may make a server allocate is declared, not inherited from the traffic.** `ServerLimits` is `#[require]`d by `HttpServer` and read once per accept loop, so every socket-backed server (mini and hyper) answers under a cap: `max_in_flight` permits taken around the *dispatch* (not the connection, so a slow client holds nothing back), `max_body_bytes` refusing an oversized `content-length` with a `413` before a byte of the body is read or allocated, and `dispatch_timeout` answering `503` rather than hanging. Override per server, `<HttpServer {ServerLimits{max_in_flight:4}}/>`. These are remote-OOM guards, not tuning knobs: both halves were live defects that killed `beet.org` (2026-09-30), an unbounded accept loop spending ~13 MB per in-flight request for the dearest route and a body read sized from the client's own header. Lower `max_in_flight` is *faster* as well as lighter, since a page builds on the one world thread and concurrency past a handful buys queueing rather than throughput. The one hazard it creates is a handler that calls its own server (the charcell image fetch reaches the canonical loopback port like any client): it holds a permit while waiting for a second, which is what `dispatch_timeout` exists to break. A loopback exemption is not the answer — a deployed box is reverse-proxied from `localhost`, so every real request is loopback.

**What a request may carry is declared too.** An extractor declares a param by taking it (an action's `Params<T>` argument or input, `AnyParams` for every param), landing a `ParamsDeclaration` in `RouteParams`, read by the route it sits on, or `SubtreeParams`, read by every route beneath it, as each server does for its own (`--accept` and `--server` on a `CliServer`, the bind knobs on the others). A route's union of those is its `ParamsPattern`, and `RejectUnknownParams`, which every `CliServer` requires, refuses whatever that pattern does not hold; how a route's params are collected, refused and listed is `beet_router`'s README, Params.

## Stores and the repo store

A store holds bytes or rows, and neither implies the other: a `BlobStore` is the erased handle every blob backend (`FsStore`, `S3Store`, `InMemoryStore`, `SqliteBlobStore`, the browser stores, the read-only `HttpStore` over a served `<ServeBlobs>` mount) materializes on its entity, and a `TableStore` (Tables, below) every table backend's, so a consumer never names a backend and never reaches the wrong kind. Both are a `StoreBackend`, an identity and a lifecycle. A backend is selected by a `StoreUri` (`fs:<path>`, `s3://<bucket>`, `dynamo://<table>`, `sqlite:<path>`, `indexed-db://<db>`, `http:<prefix>`, each carrying its own root) as the kind that is asked for, `StoreProvider::from_uri` building a blob store and `TableStoreProvider::from_uri` a table store, the one place a uri becomes a concrete store component: `sqlite:` answers both, `dynamo:` a table alone, and a blob uri asked for as a table store is a `JsonTableStore` over it; `StoreProvider::compose` forks any of them into a `StoreFork` (a local store reads try first and writes land in), the fork a process keeps of a repo it reads but does not own, the store-grain twin of `SceneFork`; in a browser the fork's first write sets one local-storage bit (`StoreUri::fork_mark`) a served page reads before its world exists, to hide the published page from a returning editor. Exactly one store in an app is the **repo store**: the canonical store an entry loads through, marked `RepoStore` and rooted on the entry root, holding the entry document, its templates, routes and assets. A relative path anywhere in the app means a path in it, and a second one anywhere in the world is an error.

Consumers usually resolve it by ancestry (the `AncestorQuery<&BlobStore>` idiom, which honours any intervening `DirPath` scope); `RepoStore::get` is the direct lookup. Every other store is a plain store, declared for a purpose and reached by name through a `StoreRef`. A consumer of several stores names them one field per role in one place instead — its own `#[field(required)] #[entities]` fields where it is an action (`AnalyticsRollupJob{analytics, archive}`), one shared component where several consumers need the same set — since a generic `StoreRef` beside a specifically named twin leaves the generic one silently role-bearing. See `src/store/mod.rs` for the backends and the reactive `BlobEvent` substrate. `StoreMirror` is the one sync between any two, behind `<StoreSync/>` and `beet store-sync` (`src/store/store_sync.rs`).

A derived file is a document computed from declared sources and stored beside the `Provenance` naming them: a `Derivation { path, ttl }` declares one beside the `Action<BlobStore, EntityBody>` producing it, and `Derivation::ensure` regenerates it only when a source changed, its ttl ran out or it is missing (`src/store/derivation.rs`; the pattern is `Provenance`'s module docs in `beet_core`). `beet_infra`'s `<Derive/>` step ensures every one a document declares, except one naming the verb that alone writes it (`Derivation::written_by`, ie the standard site index `publish` writes), which a deploy ships as committed.

`src/store_actions/` is what markup and an agent toolset reach those stores through: `ListBlobs`/`ReadBlob`/`WriteBlob`/`WriteText`/`EditText`/`RemoveBlob` over the nearest ancestor `BlobStore`, and `SqlSelect`, one authored `SELECT` over the `SqliteStore` a `StoreRef` names (`<Route path="senders" {(SqlSelect{sql:".."}, StoreRef($index))}/>`). There is no write twin of `SqlSelect` on purpose: a SQLite store a consumer queries is an index, whose schema comes from whatever fills it and whose content is rebuilt from that source.

`SqliteStore` runs on every target, browsers included: `rusqlite` builds its ffi from `sqlite-wasm-rs` rather than `libsqlite3-sys` on `wasm32-unknown-unknown`, so the SQL dialect, the generated columns and the query planner are the same engine everywhere. Two things differ there and both are the default VFS holding the database in memory rather than in a file: the store exists while a connection to it does (there is nothing to stat), and there is no WAL. Nor is there a thread to hand a query to, so a call runs where it is made — which is why a browser build that cares about jank, or wants a durable OPFS VFS, belongs in a dedicated worker.

## Tables

`TableStore` is beet's one dialect for table-shaped data (`src/store/table/`), rows under ordered keys; bytes stay a `BlobStore`'s and anything shaped otherwise (a graph, a search index, a time series) its own store's. A table store is declared, by name, and a consumer finding none at or above it is refused naming what asked and how to declare one (`TableStore::nearest`, `TableStore::declared`), never given a fallback. `<FsStore path="target/chat" {JsonTableStore}/>` is the naive one, json files over the nearest blob store, for trying things in a small binary and for a browser tab; a table that grows is a `SqliteStore` or a deployed database. A **row** is a map of named fields (`Row`), keyed within its table by a `TableKey` that orders by raw bytes on every backend; a `Tid` is the default key, its alphabet's byte order its time order. Each row carries the **cid** of its DAG-CBOR with floats permitted, computed once when it is built and carried with the row (`StoredRow`) from then on, so a float-free row with `$type` has exactly its record's cid.

- **Typed rows.** `Table<T: TableStoreRow>` (`store.table::<T>()`): `get`, `get_with_cid`, `put` (any), `push` (absent), `put_if`, `remove`, `remove_if`, `scan` (bounds, limit, direction, cursor; one page), `scan_index`, and `list`/`get_all`/`scan_all` paging internally. A row type names its table, key and secondary indexes (`TableStoreRow::indexes`).
- **Preconditions.** Every write takes a `Precondition`, `Any`, `Absent` or `Matches(cid)` (atproto's `swapRecord`), checked and applied as one step; a failed one is `TableError::Conflict`, what a read-modify-write retries on, so two writers never silently erase each other.
- **Partitions.** `store.partition(id)` scopes one database out of a declaration the way `with_subdir` scopes a blob store: one writer, atomic within itself, isolated from every other. `apply` (an atomic batch across tables) lives on the partition, so a batch never spans two.
- **Record tables.** A table named by an NSID is a record table: `Table<Rkeyed<T: AtprotoRecord>>`, the key an rkey, the row the record with its `$type` (a float the `org.beet.core#float` object). Every write to an NSID table is validated as a record, the erased `TableStore::put` included.
- **Float fidelity.** A row kept as json text is read back exactly (`Row::parse_json`), so its recomputed cid is the one it was written with. The wire carries rows through `serde_json`, which rounds some floats by a last digit, and the cid beside them, so there the cid is right where the float may not be.
- **Capabilities.** An optional capability is asked with `supports`, and a backend lacking one refuses with `TableError::Unsupported`; a fixed limit refuses with `TableError::Limit`.
- **The wire.** `TableWire` serves a partition's rows over `Request`/`Response`, `TableWireClient` speaks it to any `TableWireTransport`: `<ServeTables prefix=".."/>` over http, a `PartitionHost` in process, a Durable Object's stub fetch. `TableArchive` moves a key range of cold rows into a SQLite file in a bucket and retires them.

| backend | atomic batches | secondary indexes | sql | a partition is | limits |
| --- | --- | --- | --- | --- | --- |
| `JsonTableStore` over a blob store (memory, fs, S3, R2, IndexedDB, local storage, `SqliteBlobStore`) | yes, the file replaced while it is the one read | yes, a sort | no | its own file, `<id>.json` | the blob store's object size; every operation reads its partition's file and a write rewrites it |
| `SqliteStore` | yes, one transaction | yes, `json_extract` expression indexes | yes, `execute`/`query`/`SqlSelect` | its own file, `<dir>/<stem>/<id>.db` | none of its own |
| `DynamoStore` | yes, `TransactWriteItems` | yes, four generic slots, eventually consistent | no | its `pk` | 100 writes a batch, 4 indexes a row type, 400KB an item |
| `DurableObjectStore` (natively a `PartitionHost`) | yes, every precondition checked before any write | yes, as SQLite | no | its own Durable Object, natively its own file | 2MB a row; an integer past 2^53 binds as a float |
| `TableWireClient` | the server's | the server's | no | the `@<id>` path prefix | the server's |
| a PDS (the scene compiler's `PdsStore`) | `applyWrites` under `swapCommit` | no | no | a repo | record tables only |

## AWS

The `aws` feature is AWS as Signature Version 4 requests through this crate's own client (`src/client/aws/mod.rs`): `AwsClient` over every protocol beet speaks, the credential chain, STS, the S3 and DynamoDB stores, the Lambda runtime loop (`lambda`), and IAM Identity Center's sign in (`IdentityCenterClient`). No SDK, no async runtime and no `aws` cli, so the same code signs from a native binary, a Lambda and a Cloudflare Worker. Where each piece runs:

| | native | Cloudflare Worker | browser |
|---|---|---|---|
| credentials | explicit, environment, shared file, ECS container, EC2 instance metadata | explicit, environment (the entry seeds it from the Worker's secret bindings) | explicit, environment |
| `AwsClient`, every store and api call | yes | yes, within a request's subrequest and memory caps | compiles; nothing is built for it (no presigned urls, no CORS) |
| `beet admin`, `beet aws`, Identity Center sign in | yes | no person to ask | no |
| `logs tail --follow` (`<AwsWatch/>`) | yes | unsuited: a request ends | no |
| the Lambda runtime (`lambda`) | in a Lambda task | no | no |
| `<stack>/deploy` | yes | no: it runs `tofu` and `cargo` | no |

The operator's own `aws` cli is reachable through `beet aws -- <args>` (`AwsExec`), the one place beet spawns it.

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
| `lambda` | The Lambda runtime: beet's own loop over the Runtime API (`LambdaRuntime`) |
| `json` | Tables (`TableStore`, `JsonTableStore`, the wire, `DurableObjectStore`), rows hashed through `dag_cbor` |
| `sqlite` | `SqliteStore`, `SqliteBlobStore`, `PartitionHost` and `TableArchive`, every target |
| `cloudflare` | The Worker runtime's stores: `R2WorkersStore` and `DurableObjectStore`'s objects (wasm only) |
| `aws` | AWS over signed requests (`AwsClient`), the S3 and DynamoDB stores, STS and Identity Center, see AWS above |
| `oauth` | The reusable pieces of an OAuth client: PKCE, state, the loopback redirect, the device grant, the token set |
| `reqwest` | Use reqwest as the HTTP client backend |
| `ureq` | Use ureq as the HTTP client backend |
| `tungstenite` | Native WebSocket support |
| `russh_client` / `russh_server` | SSH client and server |
| `webdriver` | WebDriver browser automation client: BiDi sessions, auto-waiting finds, trusted input (a secret typed without a trace), console/network collectors, cookies, persistent profiles, screenshots and PDF export |
| `mdns` / `udp` | mDNS service discovery and UDP sockets |
| `rustls-tls` | Use rustls for TLS |
| `native-tls` | Use native TLS implementation |
| `secure` | TLS serving: the `Tls` component (self-signed dev cert or provided PEM files) |
| `atproto` | An account's repo on a real PDS: `XrpcPds` writing through `AppPassword` (a `RequestAuth`), did and handle resolution, and the `app.bsky.*` lexicons beet writes (`FeedPost`, `RichText`). The `Pds` family itself (`EmulatorPds`, the converge, `<AtprotoAccount/>`) rides `json`; see `src/atproto/mod.rs` |