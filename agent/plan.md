# beet_net

- fix feature gates for `cargo test -p beet_net --lib --target=wasm32-unknown-unknown`, ie without all-features
- beet_net/src/client/send.rs -> sending via file *is* valid in wasm, fs_ext supports wasm targets via our own js runtimes.
- remove the `fs` feature flag gating for this, thats actually just for async stuff. path_clean should always be included.
- impl_file.rs streaming was implemented incorrectly, use `fs_ext::read_stream`


verify all working with `just test-all | tail`
