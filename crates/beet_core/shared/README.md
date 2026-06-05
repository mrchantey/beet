# beet_core_shared

Token-manipulation utilities shared by `beet_core` and the `beet_core_macros` proc-macro crate.

A proc-macro crate cannot share modules with a regular library crate, so the `syn`/`quote` helpers both need live here. You should not depend on this crate directly; it is re-exported through `beet_core` behind the `tokens` feature.

Contents:

- `AttributeGroup` / `AttributeMap` - parse and validate `syn` attributes
- `NamedField` - struct fields and function parameters with attribute parsing
- `TokenizeSelf` - tokenize a value back into its constructing source
- `pkg_ext` - Cargo.toml helpers for resolving internal vs external crate paths
- `synbail!` / `synhow!` - ergonomic `syn::Error` construction
