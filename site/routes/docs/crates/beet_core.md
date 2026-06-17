+++
title = "beet_core"
+++

# beet_core

`beet_core` is the floor the rest of beet stands on. It is `no_std` by default and re-exports the handful of types and traits the workspace leans on, so most crates open with a single `use beet_core::prelude::*`.

Its job is to paper over the differences between native, wasm and embedded targets before anything else has to think about them. Where the standard library would tie you to one platform, beet_core offers cross-platform stand-ins: `fs_ext`, `env_ext` and `time_ext` behave the same everywhere, which is why beet code never reaches for `std::fs` or `std::env` directly.

It also sets the house style. The `xtend` traits add method-chaining helpers like `xmap` and `xok` so logic reads top to bottom instead of inside out, and the re-exported `HashMap`, `HashSet` and `Instant` are tuned for beet's needs rather than cryptographic defaults.

```rust
# use beet_core::prelude::*;
let total = vec![1, 2, 3]
	.xtap(|nums| assert_eq!(nums.len(), 3))
	.into_iter()
	.sum::<i32>();
assert_eq!(total, 6);
```

Finally, beet_core owns testing. The `#[beet_core::test]` attribute and its chainable matchers (`xpect_eq`, `xpect_true`, and friends) run the same way on native and wasm, so a single test suite covers every target. The matchers are a readability choice, not a replacement for `unwrap`, which tests still use to fetch values.
