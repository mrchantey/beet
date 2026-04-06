### Testing

The most important part! there are lots of leftover bits and pieces that need cleaning up and fixing. we need to do a thorough shakedown, especially of `beet_node` and `beet_router`.

Run the below tests in sequence, check them of one by one as they pass.

`cargo test -p beet_node | tail`
`cargo test -p beet_node --no-default-features | tail`
`cargo test -p beet_node --all-features | tail`
`cargo test -p beet_node --all-features --lib --target=wasm32-unknown-unknown | tail`
`cargo test -p beet_router --all-features | tail`
`cargo test -p beet_router --all-features --lib --target=wasm32-unknown-unknown | tail`
`just test-core | tail`

Do not ignore pre-existing issues, fix them.

then also verify the examples are building, and run them with a timeout, many will never exit.
