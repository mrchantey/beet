# beet

<div align="center">
  <p>
    <strong>The Bevy Expansion Pack</strong>
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

Beet is a very bevy metaframework, bringing bevy patterns *and* principles (thats where the 'very' comes in) to the rest of the stack.

Thats right fam, we're going full-stack bevy. Its *very* early days so your mileage may vary depending on your application:

**readiness meter**
- 🌳 documented and tested
- 🌿 docs and tests are incomplete
- 🌱 highly experimental, germinating

I think of the beet crates as divided into a few main categories.

## Utils

General patterns and tools for application development.

| Crate                                            | Status | Description                                     |
| ------------------------------------------------ | ------ | ----------------------------------------------- |
| [`beet_utils`](crates/beet_utils/Cargo.toml)     | 🌱      | Absolute base level utility crate               |
| [`beet_core`](crates/beet_core/Cargo.toml)       | 🌱      | Core utilities and types for other beet crates  |
| [`sweet`](crates/sweet/Cargo.toml)               | 🌿      | A pretty cross platform test runner             |
| [`sweet-cli`](crates/sweet/cli/Cargo.toml)       | 🌿      | A pretty cross platform test runner             |


## Beet Flow

Control flow crates for use in behavior paradigms like Behavior Trees, LLMs and Reinforcement Learning.

| Crate                                            | Status | Description                                                       |
| ------------------------------------------------ | ------ | ----------------------------------------------------------------- |
| [`beet_flow`](crates/beet_flow/Cargo.toml)       | 🌳      | An ECS control flow library                    |
| [`beet_spatial`](crates/beet_spatial/Cargo.toml) | 🌿      | Spatial actions built upon beet_flow           |
| [`beet_ml`](crates/beet_ml/Cargo.toml)           | 🌱      | Machine Learning actions built upon beet_flow  |
| [`beet_sim`](crates/beet_sim/Cargo.toml)         | 🌱      | Game AI simulation primitives.                 |


## Beet Rsx

Crates for building and deploying web apps.

| Crate                                          | Status | Description                                  |
| ---------------------------------------------- | ------ | -------------------------------------------- |
| [`beet_rsx`](crates/beet_rsx/Cargo.toml)       | 🌱      | A rust/bevy implementation of jsx dom interaction |
| [`beet_rsx_combinator`](crates/beet_rsx_combinator/Cargo.toml) | 🌱      | JSX-like parser combinator for Rust          |
| [`beet_parse`](crates/beet_parse/Cargo.toml)   | 🌱      | Parsers for various text and token formats   |
| [`beet_build`](crates/beet_build/Cargo.toml)   | 🌱      | Codegen and compilation tooling              |
| [`beet_net`](crates/beet_net/Cargo.toml)       | 🌱      | Cross-platform networking utilities          |
| [`beet_design`](crates/beet_design/Cargo.toml) | 🌱      | Design system and components for beet rsx    |
| [`beet-cli`](crates/beet-cli/Cargo.toml)       | 🌱      | Tools for building and deploying beet apps   |
| [`beet_site`](crates/beet_site/Cargo.toml)     | 🌱      | The beet website, built with beet            |


## Experiments

| Crate                                            | Status | Description                                               |
| ------------------------------------------------ | ------ | --------------------------------------------------------- |
| [`beet_agent`](crates/beet_agent/Cargo.toml)     | 🌱      | Bevy-friendly patterns for interaction with agents       |
| [`beet_query`](crates/beet_query/Cargo.toml)     | 🌱      | Extend beet server actions with database queries         |
| [`beet_examples`](crates/beet_examples/Cargo.toml) | 🌱      | bits and pieces for substantial beet examples            |
| [`emby`](crates/emby/Cargo.toml)                 | 🌱      | the beetmash ambassador                                   |
| [`beet_mcp`](crates/beet_mcp/Cargo.toml)         | 🌱      | Experimental mcp server                                   |


## Bevy Versions

This chart is for matching a bevy version against a particular beet version.

| `bevy` | `beet` |
| ------ | ------ |
| 0.16   | 0.0.6  |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |
