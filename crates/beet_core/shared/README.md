# beet_core_shared

Shared utilities for `beet_core` and `beet_core_macros`.

This crate contains token manipulation utilities that need to be compiled for both the main `beet_core` crate and the `beet_core_macros` proc-macro crate.

## Purpose

Proc-macro crates have special compilation requirements and cannot share code directly with regular library crates through module paths. This crate solves that problem by extracting shared token utilities into a standalone crate that both `beet_core` and `beet_core_macros` can depend on.

## Contents

- **`AttributeGroup`** - Parse and validate syn attributes
- **`NamedField`** - Wrapper for struct fields and function parameters with attribute parsing
- **`pkg_ext`** - Package configuration helpers for determining internal vs external crate usage

## Usage

Users typically don't need to depend on this crate directly. It is reexported through `beet_core::tokens_utils` when the `tokens` feature is enabled.

```rust
use beet_core::prelude::*; // With tokens feature enabled

let attrs = AttributeGroup::parse(&field.attrs, "field")?;
```
