
Prepare the speficied crates for release, for each crate follow the below procedure.

## 1. Documentation
do
Gain an understanding of the intended usage of the crate. Do not comb through every single source file, just check the high level types and ensure they are well documented. Ensure the `README.md` is up to date and of high quality. The `lib.rs` should include the readme as crate level docs.

## 2. Testing

Run checks and tests for the crate with all features, both native and wasm:
```sh
cargo check -p CRATE_NAME --all-features
cargo check -p CRATE_NAME --all-features --target=wasm32-unknown-unknown
timeout 1m cargo test -p CRATE_NAME --all-features | tail 30
timeout 1m cargo test -p CRATE_NAME --all-features --target=wasm32-unknown-unknown --lib | tail 30
```

## 3. Examples

All examples are located in `examples/CRATE_NAME`, not `crates/CRATE_NAME/examples`. Ensure an example exists for each intended usage of the crate. If one is missing create it.

For each example:
- check the implementation is up to date.
- ensure the example is well and consistenly documented, include the command to run the example, specifying features.
- run the example to verify it works, ie:
	- `timeout 1m cargo run --example server --features=server | tail 30` (then simultaneously run a `curl`, waiting for it to be live)
