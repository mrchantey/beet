# Time for a rebuild


This repo is massive, its common to get weird mold compile errors. when they get too bad we do a full rebuild

## 1. Clear All

- run `just clear-all`
Absolutely clears all artefacts

## 2. Try full test suite

- run `just test-all`, use tail and 10m timeout to preserve context

The first few times this usually gets some compile errors like out of stack space, in those cases we can just run it again
if that still doesnt work a neat trick i sometimes use is bump the versions of all workspace crates in the root cargo.toml, ie bump all the `0.0.10-dev.3` just to unstuck the compiler
keep trying and debugging any compiler issues. keep in mind we are using `clang/mold` which is much faster than default but also more unstable, compiler issues like `increase stack size` are usually resolved by simply running again.


## Current State

So far no good.

we have recently added some features to `beet_core` which have resulted in strange compiler issues, namely splitting `serde` feature into `json` and `serde`, as well as adding the `postcard` feature.

To verify note:
`cargo test -p beet_core --lib` OK
`cargo test -p beet_core --lib --features=serde` OK
`cargo test -p beet_core --lib --features=json` compile error

**We have tried**:
- different linkers: same issue
- different nightly compilers: same issue
- renaming libtest import: same issue
- rust stable wont help, custom test runners is nightly only
This is definitely an issue with some change in the codebase, likely due to these recent refactors involving json, postcard, and possibly the introduction of the `exchange_format.rs`

You may clear as often as needed, continue investigating to find the root cause of this compiler issue.

once this issue is cleared, continue to verify the rest of the codebase with `just test-all` (naturally with tail)


**TO BE CLEAR**
- do not mess with compiler flags or versions, we've exhausted this
- only investigate by changing stuff in the codebase. for instance, commenting out everything in `beet_core/src/lib.rs` resolves the issue, so maybe start there and binary search until you find the culprit
