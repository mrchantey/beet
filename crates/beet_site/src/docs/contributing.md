+++
title= "Web"
+++


The web metaframework features are highly experimental. Running test builds often results in a rust compiler stack overflow the first few times,

## Testing

Here's the requirements for verifying everything works.

```sh
# currently nightly is a requirement
rustup default nightly
# install prebuilt binaries
cargo install cargo-binstall
# wasm builds
rustup target add wasm32-unknown-unknown
# we need exact bindgen versions
cargo binstall --no-confirm wasm-bindgen-cli --version=0.2.100
# only required for deploying
cargo binstall --no-confirm wasm-opt
# install nix for dom testing
sh <(curl --proto '=https' --tlsv1.2 -L https://nixos.org/nix/install) --daemon
# Command runner
cargo binstall just
# Test runner
just install-sweet
# Run all tests
just test-rsx
```

### Troubleshooting


#### Compile Errors

Building beet from scratch often results in stack overflows, you may get an error like:
```
help: you can increase rustc's stack size by setting RUST_MIN_STACK=2147483648
```
First try running it again, often it will get there after the first few tries. If it compiles a few more crates then overflows again, keep going its getting there :)

If it repeatedly fails without getting any closer it could be the linker has some malformed objects. I find incrementing all the beet versions in `Cargo.toml`, ie replace all `0.0.7-dev.8` with `0.0.7-dev.9`, is a nice way to help it out without a full `cargo clean`.
