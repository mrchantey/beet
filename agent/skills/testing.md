### Testing

The most important part! there are lots of leftover bits and pieces that need cleaning up and fixing. we need to do a thorough shakedown, especially of `beet_ui` and `beet_router`.

Run the below tests in sequence, check them of one by one as they pass.

`cargo test -p beet_ui | tail`
`cargo test -p beet_ui --no-default-features | tail`
`cargo test -p beet_ui --all-features | tail`
`cargo test -p beet_ui --all-features --lib --target=wasm32-unknown-unknown | tail`
`cargo test -p beet_router --all-features | tail`
`cargo test -p beet_router --all-features --lib --target=wasm32-unknown-unknown | tail`
`just test-core | tail`

Do not ignore pre-existing issues, fix them.

then also verify the examples are building, and run them with a timeout, many will never exit.
