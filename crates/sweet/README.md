# Sweet

A cross platform extension of the default test runner.

## Usage

```rust
// Optionally use the sweet test runner for wasm support, pretty output etc
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use sweet::prelude::*;

#[test]
fn it_passes(){
	// regular assertions work as expected
	assert!(1 + 1 == 2);
	// type-specific matchers are also available
	"sweet as".xpect_contains("sweet");
}

// async functions are also supported
#[sweet::test]
async fn foo(){ .. }

```
And then run as normal
```sh
cargo test
```

## Sweet Runner

The sweet runner unlocks full custom test framework features like wasm tests, snapshotting and pretty output. Note that custom test frameworks are a [nightly feature](https://github.com/rust-lang/rust/issues/50297).

There are three steps to setting up the sweet runner:
1. Enable the feature in `Cargo.toml`
	```toml
	[dev-dependencies]
	sweet = { version = "..", features = ["runner"] }
	```
2. Update the workspace `.cargo/config.toml` to point to the sweet cli, and set the root env var.
	```toml
	[target.wasm32-unknown-unknown]
	# wasm requires the cli: `cargo binstall sweet-cli`
	runner = 'sweet run-wasm'
	[env]
	SWEET_ROOT = { value = "", relative = true }
	```
3. Add the following attributes to binary entrypoints like `src/main.rs`.
	```rust
	#![cfg_attr(test, feature(test, custom_test_frameworks))]
	#![cfg_attr(test, test_runner(sweet::test_runner))]
	```

Then run tests as normal, including on wasm.
```sh
cargo test -p my_crate
cargo test -p my_crate --target wasm32-unknown-unknown
```

## Features

### `runner`

Enables the sweet runner and internal dependencies. With this disabled the runner falls back to the built-in libtest runner.

## Contributing

