# beet

<div align="center">
  <p>
    <strong>🦄 The Unistack Metaframework 🦄</strong>
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

Beet is a set of crates for making and running Bevy apps.
Its *very* early days so your mileage may vary depending on the crate of interest:

**readiness meter**
- 🦢 documented and tested
- 🐣 docs and tests are incomplete
- 🐉 highly experimental, here be dragons

## `beet_flow`

Control flow crates built upon the [ecs engine](https://crates.io/crates/bevy_ecs) that powers Bevy. These can be used for a growing variety of behavior paradigms including Behavior Trees, LLMs and Reinforcement Learning. They are also decoupled from rendering, for instance they can be run on small microcontrollers like the ESP32.

| Crate                                            | Status | Description                                                       |
| ------------------------------------------------ | ------ | ----------------------------------------------------------------- |
| [`beet_flow`](crates/beet_flow/Cargo.toml)       | 🦢      | Scenes-as-control-flow bevy library for behavior trees etc        |
| [`beet_spatial`](crates/beet_spatial/Cargo.toml) | 🐣      | Extend `beet_flow` with spatial behaviors like steering           |
| [`beet_ml`](crates/beet_ml/Cargo.toml)           | 🐉      | Extend `beet_flow` with machine learning using `candle`           |
| [`beet_sim`](crates/beet_sim/Cargo.toml)         | 🐉      | Extend `beet_flow` with generalized simulation tooling like stats |


## `rsx`

An exploration of a rusty `jsx`, and the tools required to maximize performance and developer productivity. 

| Crate                                          | Status | Description                    |
| ---------------------------------------------- | ------ | ------------------------------ |
| [`beet_rsx`](crates/beet_rsx/Cargo.toml)       | 🐉      | Cross domain authoring tools   |
| [`beet_router`](crates/beet_router/Cargo.toml) | 🐉      | File based router for websites |

## `sweet`

General utilities including a test runner, file watcher etc.

| Crate                                                             | Status | Description                            |
| ----------------------------------------------------------------- | ------ | -------------------------------------- |
| [`beet_bevy`](https://crates.io/crates/beet_bevy)                 | 🐉      | Bevy utilities                         |
| [`beet_server_utils`](https://crates.io/crates/beet_server_utils) | 🐉      | Simple file server with live reload    |
| [`sweet`](https://crates.io/crates/sweet)                         | 🐣      | A pretty cross platform test runner    |
| [`sweet-cli`](https://crates.io/crates/sweet-cli)                 | 🐣      | Cross-platform utilities and dev tools |


## `crates`

Top level crates that depend on several of the above.

| Crate                                           | Status | Description                  |
| ----------------------------------------------- | ------ | ---------------------------- |
| [`beet-cli`](https://crates.io/crates/beet-cli) | 🐉      | CLI for beet authoring tools |
| [`beet_mcp`](https://crates.io/crates/beet_mcp) | 🐉      | VectorDB MCP Server          |


## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.16   | 0.0.6  |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md)
