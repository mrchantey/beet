---
title: Quick Start
description: Getting up and running with sweet.
draft: true
sidebar:
  order: 1
---

Configure sweet by adding two lines to your entrypoint:

```rust title="src/lib.rs"
// sweet uses the custom_test_frameworks nightly feature
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[test]
fn my_test(){
	assert!(1 + 1 = 2);
}
```

```sh
cargo add sweet
cargo test
```

## Wasm

All `#[test]` and `#[sweet::test]` tests can run in wasm by configuring the sweet runner:

```toml title=".cargo/config.toml"
[target.wasm32-unknown-unknown]
runner = 'sweet test-wasm'
```
```sh
cargo binstall deno wasm-bindgen-cli sweet-cli
cargo test --target wasm32-unknown-unknown
```