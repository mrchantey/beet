---
title: Sweet
description: Delightful Rust testing
draft: true
sidebar:
  label: Overview
---

The sweet test runner is built from the ground up with web dev in mind, the `#[sweet::test]` macro will run any test, be it sync, async, native or wasm.

It unifies native and wasm async tests, integrates with existing `#[test]` suites and outperforms other runners.

## Performance

- Sweet matchers compile [100x faster](./assert.md) than `assert!` macros.
- `#[sweet::test]` outperforms `#[tokio::test]` through shared async runtimes.

## Versatility

There is a range of use-cases beyond the humble `#[test]` and that has led to the creation of several test crates:
- `#[wasm_bindgen_test]` provides wasm support for both standard and async testing.
- `#[tokio::test]` allows for isolated async tests.
- `#[tokio_shared_rt::test]` adds the option of a shared tokio runtime, with resource sharing and faster startup times.

Sweet supports all of these under a single `#[sweet::test]` macro, and it also collects and runs `#[test]`, `#[tokio::test]`, etc. [^1]

## The 1 Second Rule

A correctly formatted test output should give the developer an intuition for what happend in less than a second, with verbosity flags for when its needed. Here is how sweet's approach differs from the default runner:

1. **Clarity:** The most important part of an output is *the result*, so it goes before the test name.
2. **Brevity:** File paths are shorter than fully qualified module names, increasing readability and reducing likelihood of line breaks.
4. **Linkability:** The use of file paths turns the test output into a sort of menu, viewing a specific file is a control+click away.
3. **Organization:** Files are the goldilocks level of output between individual test cases and entire binaries, so thats the default suite organization and unit of output.

```rust
// default output
test my_crate::my_module::test::my_test ... ok
// sweet output
PASS src/my_module.rs
```

## Inspiration

If the tagline or runner output format look familar that because they are copied verbatum from my favourite test runner, [Jest](https://jestjs.io/).
The handling of async wasm panics was made possible by the wizards over at `wasm_bindgen_test`.

[^1]: An exception to this is `#[wasm_bindgen_test]` because it uses a custom runner.
