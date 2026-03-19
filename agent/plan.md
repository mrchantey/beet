# Stable Rust

We're working on the `custom_test_framework` feature, where previously the beet_core test runner was nightly only, and now by default uses `inventory::collect`, gating the nightly support for regular `#[test]` functions behind `custom_test_framework`.

I have no idea whether this refactor was successfu, running `cargo test -p beet_core --lib` currently just runs libtest, not our custom runner. you can tell because the output is standard test output, not our pretty `PASS` `FAIL` format.

- `logger.rs, suite_outcome.rs` why are these tests custom framework only? this is an indication our refactor is incorrect.
- `test_plugin.rs` nightly should collect inventory as well?
- `test_test.rs` again, why nightly only for the slow one? unignore it and ensure it works. i think all of the beet_core integration tests should have a cfg_attr(feature="custom_test_framework",test_runner(beet_core::test_runner_nightly)) etc so we can run them both in our nightly and stable rust, keep the `custom_test_framework.rs` one that only runs for nightly, nice to have.
- `test_attr.rs` - to get the beet_core path we used to just have `let beet_core = pkg_ext::internal_or_beet("beet_core");` im skeptical about these special case changes to the macro. beet_core integration tests are such an edge case, just use aliases or special imports in each of them, instead. i could be wrong, but find out if we can revert to the simpler approach.
- `gen_inventory_entry` the spans are completely wrong, we already have the spans for the ItemFn, should be able to hardcode those instead of wiffing by using `line!()` etc.
- We've also completely disregarded the test attributes like #[ignore] and #[should_panic], ensure they are implemented.
- `inventory` should be an optional import

## Broken

Also fix the broken tests, run all tests using `timeout` and `tail`.

- `cargo test -p beet_core --lib` - runs libtest not our custm test framework
- `cargo test -p beet_core --lib --target=wasm32-unknown-unknown` - some very weird error
- also variations:
```
cargo test -p beet_core --lib --all-features
cargo test -p beet_core --lib --target=wasm32-unknown-unknown --all-features
just test-core
```

These were all working before the refactor and are serious regressions, fix them.
