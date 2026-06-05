# beet_core

Cross-platform foundations shared by every other beet crate.

`beet_core` is `no_std` by default and re-exports the types and extension traits beet leans on, so most crates only need `use beet_core::prelude::*`. Highlights:

- **Cross-platform std replacements**: `fs_ext`, `env_ext`, `time_ext` and friends that work the same on native, wasm and embedded targets.
- **Method-chaining extensions**: the `xtend` traits (`xmap`, `xtap`, `xok`, ...) for fluent code, plus fast `HashMap`/`HashSet`/`Instant` re-exports tuned for beet.
- **Test runner**: the `testing` module provides the `#[beet_core::test]` attribute and chainable matchers (`xpect_eq`, `xpect_true`, ...) used across the workspace.
- **Bevy extras**: extension traits, async commands and the vendored async-world bridge ([`beet_async`]).

```rust
# use beet_core::prelude::*;
let total = vec![1, 2, 3]
	.xtap(|nums| assert_eq!(nums.len(), 3))
	.into_iter()
	.sum::<i32>();
assert_eq!(total, 6);
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `std` | Standard library support (enabled by default) |
| `serde` | Serialization support |
| `testing` | Test runner and matcher utilities |
| `fs` | File system watching and utilities (native only) |
| `tokens` | Proc-macro token utilities |
| `rand` | Random number generation |
| `nightly` | Nightly-only features like `Fn` trait impls |
