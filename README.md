# beet

<div align="center">
  <p>
    <strong>A personal application framework</strong>
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
    <!-- <span> | </span>
    <a href="https://mrchantey.github.io/beet/other/contributing.html">Contributing</a> -->
  </h3>
</div>

Beet is a framework for building technologies that can people can share, access and make their own like other forms of folk culture like music and story. Beet uses the Entity Component System architecture of Bevy, a modular game engine, to provide a common agnostic paradigm for state and behavior across domains like web, games and robotics.

> 🚧 Mind your step! 🚧
>
> Beet is under construction, if this project is of interest please come and say hi in the [Beetmash Discord Server](https://discord.gg/DcURUQCXtx).

**readiness meter**
- 🦢 ready to go: documented and tested
- 🐣 near stable: incomplete docs
- 🐉 highly experimental: here be dragons

Beet crates fall into a few main categories.

## Utils

General patterns and tools for application development.

| Crate                                        | Status | Description                                    |
| -------------------------------------------- | ------ | ---------------------------------------------- |
| [`beet_core`](crates/beet_core/Cargo.toml)   | 🦢      | Core utilities and types for other beet crates |
| [`beet_net`](crates/beet_net/Cargo.toml)     | 🐣      | Very bevy networking utilities.
            |


## Control Flow

Control flow crates for use in behavior paradigms like behavior trees, utility AI or agentic systems.

```rust
world
  .spawn((
    Name::new("My Behavior"),
    Sequence,
    children![
      (
        Name::new("Hello"),
        EndWith(Outcome::Pass),
      ),
      (
        Name::new("World"),
        EndWith(Outcome::Pass),
      ),
    ],
  ))
  .trigger_target(GetOutcome)
  .flush();
```


| Crate                                            | Status | Description                                        |
| ------------------------------------------------ | ------ | -------------------------------------------------- |
| [`beet_flow`](crates/beet_flow/Cargo.toml)       | 🦢      | An ECS control flow library                        |
| [`beet_spatial`](crates/beet_spatial/Cargo.toml) | 🐣      | Spatial actions built upon beet_flow               |
| [`beet_ml`](crates/beet_ml/Cargo.toml)           | 🐉      | Machine Learning actions built upon beet_flow      |


## Web

Crates for building and deploying web apps. These crates are very experimental and changing frequently.

```rust
#[template]
fn Counter(initial: i32) -> impl Bundle {
  let (get, set) = signal(initial);

  rsx! {
    <button onclick=move |_| set(get() + 1)>
      Cookie Count: {get}
    </button>
  }
}
```


| Crate                                                          | Status | Description                                         |
| -------------------------------------------------------------- | ------ | --------------------------------------------------- |
| [`beet_dom`](crates/beet_dom/Cargo.toml)                       | 🐉      | Utilities for dom rendering and interaction         |
| [`beet_parse`](crates/beet_parse/Cargo.toml)                   | 🐉      | Parsers for various text and token formats          |
| [`beet_rsx`](crates/beet_rsx/Cargo.toml)                       | 🐉      | An Astro inspired templating system built with bevy |
| [`beet_rsx_combinator`](crates/beet_rsx_combinator/Cargo.toml) | 🐉      | JSX-like parser combinator for Rust                 |
| [`beet_router`](crates/beet_router/Cargo.toml)                 | 🐉      | IO agnostic routing utilities                     |
| [`beet_build`](crates/beet_build/Cargo.toml)                   | 🐉      | Codegen and compilation tooling for beet            |
| [`beet_design`](crates/beet_design/Cargo.toml)                 | 🐉      | Design system and components for beet rsx           |
| [`beet-cli`](crates/beet-cli/Cargo.toml)                       | 🐉      | Tools for building and deploying beet apps          |
| [`beet_site`](crates/beet_site/Cargo.toml)                     | 🐉      | The beet website, built with beet                   |


## Experiments

| Crate                                              | Status | Description                                      |
| -------------------------------------------------- | ------ | ------------------------------------------------ |
| [`beet_clanker`](crates/beet_clanker/Cargo.toml)       | 🐉      | Structured context and tool calling for llms  |
| [`beet_examples`](crates/beet_examples/Cargo.toml) | 🐉      | Bits and pieces for substantial beet examples    |
| [`emby`](crates/emby/Cargo.toml)                   | 🐉      | the beetmash ambassador                          |


## Bevy Versions

This chart is for matching a bevy version against a particular beet version.

| `bevy` | `beet` |
| ------ | ------ |
| 0.17   | 0.0.7  |
| 0.16   | 0.0.6  |
| 0.15   | 0.0.4  |
| 0.14   | 0.0.2  |
| 0.12   | 0.0.1  |


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
just test-all
```
