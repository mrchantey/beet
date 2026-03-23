---
name: documentation
description: |
  Rust documentation quality pass for beet crates. Use when improving public API docs, reducing visibility scope, adding doc comments per RFC 1574, or running doc tests. Triggers: "document crate", "doc pass", "improve docs", "add rustdoc", "missing_docs", "pub visibility audit".
---

# Rust Documentation Quality Pass

Perform a comprehensive documentation and code quality pass for beet crates. The primary goal is improving public APIs; the secondary goals are comment quality and test quality.

## When to Use

- User asks to document a crate or improve docs
- User wants a `pub` visibility audit
- User enables `#![deny(missing_docs)]` and needs to fix warnings
- User asks to prepare a crate for release (documentation portion)

## Primary Goals

1. **Reduce `pub` visibility** — prefer `pub(super)`, `pub(crate)` wherever it makes sense, including at the module level and for struct fields.
2. **Public API docs** — ensure all public items are documented per RFC 1574 conventions (see below), except where a doc comment would be overly verbose.

## Secondary Goals

3. **Comments** — ensure complex logic explains its behavior.
4. **Tests** — ensure all tests are high quality.
5. **Integration tests and examples** — document these well. Examples belong in the root `examples/` directory, not inside individual crates.

## Quality Principles

- **Quality over quantity.** If a doc comment is overly verbose or explains the obvious, it has negative value. `MyStruct::new` usually needs only a one-liner; general usage goes in the top-level `MyStruct` docs.
- **Module-level docs** should contain only high-level concepts. Most modules do not need examples. Do not duplicate struct docs at the module level.
- **Preserve existing comments and TODOs** unless you are certain they are no longer relevant.

## RFC 1574 Conventions

Apply these rules when writing `///` doc comments on public Rust items.

### Summary Sentence

Every doc comment starts with a single-line summary in third-person singular present indicative, ending with a period.

```rust
/// Returns the length of the string.
/// Creates a new instance with default settings.
```

### Comment Style

Use line comments (`///`), not block comments (`/** */`). Use `//!` only for crate-level and module-level docs at the top of the file.

### Section Headings

Use these exact headings (always plural): `# Examples`, `# Panics`, `# Errors`, `# Safety`, `# Aborts`, `# Undefined Behavior`.

### Type References

Use full generic forms and link with reference-style markdown.

```rust
/// Returns [`Option<T>`] if the value exists.
///
/// [`Option<T>`]: std::option::Option
```

### Examples

Every public item should have a usage example unless the item is trivially self-explanatory.

```rust
/// Adds two numbers.
///
/// # Examples
///
/// ```
/// let result = my_crate::add(2, 3);
/// assert_eq!(result, 5);
/// ```
```

### Errors and Panics

Document what errors a function returns and under what conditions it panics.

### Safety

Required for every `unsafe` function — document all invariants the caller must uphold.

## Process

For each crate, follow these steps in order:

1. Add `#![deny(missing_docs)]` to `lib.rs`.
2. Add docs until `cargo check` passes with all features on both targets:
   - `cargo check -p <CRATE> --all-features`
   - `cargo check -p <CRATE> --all-features --target wasm32-unknown-unknown`
3. Run doc tests: `timeout 1m cargo test -p <CRATE> --doc --all-features | tail 30`
4. Mark the crate as done in the tracking file.

## Crate Order

Start from basic dependencies and work upward. Skip completed crates. Do not ask to continue to the next crate — proceed automatically.

- beet_core
- beet_flow
- beet_net
- beet_router
- beet_parse
- beet_build
- beet_dom

## Final Verification

After all crates are complete, run: `just test-all`
