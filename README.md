# beet

<div align="center">
  <p>
    <strong>Tools for developing reactive structures.</strong>
  </p>
  <p>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/v/beet.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/d/beet.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/beet"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
  <h3>
    <a href="https://bevyhub.dev/docs/beet">Guidebook</a>
    <span> | </span>
    <a href="https://docs.rs/beet">API Docs</a>
    <!-- <span> | </span>
    <a href="https://mrchantey.github.io/beet/other/contributing.html">Contributing</a> -->
  </h3>
</div>

Beet is a collection of crates for developing web pages, game scenes and AI behaviors. Your mileage may vary depending on the crate of interest:

- 🦆: documented and tested
- 🐉: highly experimental, here be dragons

| Crate          | Status | Description                                                       |
| -------------- | ------ | ----------------------------------------------------------------- |
| `beet_flow`    | 🦆      | scenes-as-control-flow bevy library for behavior trees etc        |
| `beet_spatial` | 🦆      | Extend `beet_flow` with steering, robotics etc                    |
| `beet_ml`      | 🐉      | Extend `beet_flow` with machine learning using `candle`           |
| `beet_sim`     | 🐉      | Extend `beet_flow` with generalized simulation tooling like stats |
| `beet_rsx`     | 🐉      | Exploration of authoring tools for html and bevy                  |
| `beet_router`  | 🐉      | File based router for web docs                                    |

## Bevy Versions

| `bevy` | `beet` |
| ------ | ------ |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |

