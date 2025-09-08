+++
title= "Docs"
+++

## Quickstart

> TODO update for `0.0.7`

In this quick start we will create a new website, deploy it to aws lambda and remove it.

### Build Dependencies

```sh
# install prebuilt binaries
cargo install cargo-binstall
# used by beet new
cargo binstall cargo-generate
# building wasm
cargo binstall wasm-opt
cargo binstall wasm-bindgen-cli --version=0.2.100
```
