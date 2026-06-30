# beet

<div align="center">
  <p>
    <strong>A malleable tool engine</strong>
  </p>
  <p>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/v/beet.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/d/beet.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/beet"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
  <h3>
     <a href="https://beetstack.dev">Website</a>
     <span> | </span>
    <a href="https://docs.rs/beet">API Docs</a>
  </h3>
</div>

Beet is an engine for building user-modifiable applications, like smalltalk or hypercard. Everything from the CLI to client applications is a [Bevy App](https://bevy.org), and all structure and behavior is written in Entity Component System architecture.

> 🚧 Mind your step! 🚧
>
> Beet is under construction, if this project is of interest please come and say hi in the [Beetmash Discord Server](https://discord.gg/DcURUQCXtx).

**readiness meter**
- 🦢 ready to go: documented and tested
- 🐣 near stable: incomplete docs
- 🐉 highly experimental: here be dragons

The `beet` crate re-exports the crates below behind feature flags. Each can also be used standalone.

## Core

Cross-platform primitives shared by every other crate.

| Crate                                          | Status | Description                                          |
| ---------------------------------------------- | ------ | ---------------------------------------------------- |
| [`beet_core`](crates/beet_core)                | 🦢      | Cross-platform types, extension traits and a test runner |
| [`beet_net`](crates/beet_net)                  | 🐣      | Transport agnostic request/response networking      |
| [`beet_action`](crates/beet_action)            | 🐣      | Entities as callable async functions                 |
| [`beet_ui`](crates/beet_ui)                    | 🐉      | XML-like UI trees rendered to HTML or the terminal   |
| [`beet_router`](crates/beet_router)            | 🐉      | Transport agnostic routing for bevy applications     |
| [`beet_infra`](crates/beet_infra)              | 🐉      | Infrastructure as code, built on OpenTofu            |
| [`beet_async`](crates/beet_async)              | 🐉      | Vendored bevy_async bridge for wasm and exclusive world access |

## Agents & Behavior

Behaviors built on `beet_action`, for paradigms like behavior trees, utility AI and agentic systems.

```rust
use beet::prelude::*;

# async fn run() -> Result {
let outcome = AsyncPlugin::world()
  .spawn((Sequence::new(), children![
    Log::new("hello"),
    Log::new("world"),
  ]))
  .call::<(), Outcome>(())
  .await?;
# Ok(()) }
```

| Crate                                            | Status | Description                                          |
| ------------------------------------------------ | ------ | ---------------------------------------------------- |
| [`beet_thread`](crates/beet_thread)              | 🐉      | Multi-actor orchestration for chat, humans and agents |
| [`beet_spatial`](crates/beet_spatial)            | 🐉      | Spatial actions: movement, steering and robotics     |
| [`beet_ml`](crates/beet_ml)                      | 🐉      | Machine learning actions: embeddings and RL          |

## Apps & Tooling

| Crate                                            | Status | Description                                          |
| ------------------------------------------------ | ------ | ---------------------------------------------------- |
| [`beet-cli`](crates/beet-cli)                    | 🐉      | Build, serve and run-wasm helpers for beet apps      |
| [`beet_extra`](crates/beet_extra)                | 🐉      | Extra components and systems for high-level examples |

## Bevy Versions

| `bevy` | `beet`  |
| ------ | ------- |
| 0.19   | 0.0.9   |
| 0.17   | 0.0.7   |
| 0.16   | 0.0.6   |
| 0.15   | 0.0.4   |
| 0.14   | 0.0.2   |
| 0.12   | 0.0.1   |

## Local Development

### Required Tools

- Rust nightly
- Just

### Running

Note that testing all crates involves compiling *many* crates, doing so from scratch usually results in a stack overflow in the rust compiler.
To prevent this either run with RUST_MIN_STACK='some_gigantic_number', or just keep re-running the command until its all compiled. I usually just do the latter.

```sh
git clone https://github.com/mrchantey/beet
cd beet
just init-repo
just test-core
```
