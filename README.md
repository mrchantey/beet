# beet

<div align="center">
  <p>
    <strong>🦄 The anything framework 🦄</strong>
  </p>
  <p>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/v/beet.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/d/beet.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/beet"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
  <h3>
    <!-- <a href="https://docs.rs/beet">Guidebook</a> -->
    <!-- <span> | </span> -->
    <a href="https://docs.rs/beet">API Docs</a>
    <!-- <span> | </span>
    <a href="https://mrchantey.github.io/beet/other/contributing.html">Contributing</a> -->
  </h3>
</div>

Beet is a collection of tools for building web pages, realtime applications and behaviors. Its early days so your mileage may vary depending on the crate of interest:

- 🦢 documented and tested
- 🐣 docs and tests are incomplete
- 🐉 highly experimental, here be dragons

## `ws_flow`

Control flow crates built upon the [ecs engine](https://crates.io/crates/bevy_ecs) that powers Bevy. These can be used for a growing variety of behavior paradigms including Behavior Trees, LLMs and Reinforcement Learning. They are also decoupled from rendering, for instance they can be run on small microcontrollers like the ESP32.

| Crate                                             | Status | Description                                                       |
| ------------------------------------------------- | ------ | ----------------------------------------------------------------- |
| [`beet_flow`](ws_flow/beet_flow/Cargo.toml)       | 🦢      | Scenes-as-control-flow bevy library for behavior trees etc        |
| [`beet_spatial`](ws_flow/beet_spatial/Cargo.toml) | 🐣      | Extend `beet_flow` with spatial behaviors like steering           |
| [`beet_ml`](ws_flow/beet_ml/Cargo.toml)           | 🐉      | Extend `beet_flow` with machine learning using `candle`           |
| [`beet_sim`](ws_flow/beet_sim/Cargo.toml)         | 🐉      | Extend `beet_flow` with generalized simulation tooling like stats |


## `ws_rsx`

An exploration of a rusty `JSX`, and the tools required to maximize developer productivity and performance. 

| Crate                                          | Status | Description                    |
| ---------------------------------------------- | ------ | ------------------------------ |
| [`beet_rsx`](ws_rsx/beet_rsx/Cargo.toml)       | 🐉      | Cross domain authoring tools   |
| [`beet_router`](ws_rsx/beet_router/Cargo.toml) | 🐉      | File based router for websites |

## `ws_sweet`

General utilities including a test runner, file watcher etc.

| Crate                                                   | Status | Description                            |
| ------------------------------------------------------- | ------ | -------------------------------------- |
| [`sweet_bevy`](https://crates.io/crates/sweet_bevy)     | 🐉      | Bevy utilities                         |
| [`sweet_fs`](https://crates.io/crates/sweet_fs)         | 🐉      | FS utilities                           |
| [`sweet_server`](https://crates.io/crates/sweet_server) | 🐉      | Simple file server with live reload    |
| [`sweet_test`](https://crates.io/crates/sweet_test)     | 🐣      | A pretty cross platform test runner    |
| [`sweet-cli`](https://crates.io/crates/sweet-cli)       | 🐣      | Cross-platform utilities and dev tools |


## `crates`

Top level crates that depend on several of the above.

| Crate                                           | Status | Description                  |
| ----------------------------------------------- | ------ | ---------------------------- |
| [`beet-cli`](https://crates.io/crates/beet-cli) | 🐉      | CLI for beet authoring tools |


## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.16   | 0.0.6  |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |


## Wishlist

Most of these are quite complex but if you'd like to have a go open an issue and I'll help where i can.

### `beet_rsx`
- [ ] escape html 
- [ ] reactive graph
- [ ] minify style


### `beet_router`
- [ ] markdown live reload
- [ ] markdown rsx
- [ ] markdown recursive parsing

### `sweet`
- [ ] native cli

### `beet_common`
- [ ] css parser / style tag location
- [ ] markdown parser
- [ ] file hashing

### `beet_query`
- [ ] sqlx


### `beet_server`
- [ ] sever signals

### `infra`
- [ ] serve static files on s3 instead of bundled in the lambda