# beet_core_macros

Procedural macros for `beet_core`.

These are re-exported through `beet_core` (and in turn `beet`), so depend on those rather than this crate directly. It provides:

- `#[action]` - turn a function into a callable [`Action`] (see `beet_action`)
- `#[derive(Get, Set, SetWith)]` - per-field getters and setters
- `#[derive(ToTokens)]` - tokenize a value back into its constructing source
- `#[main]` / `#[test]` - async entry point and the beet test attribute
- `rsx!` / `rsx_direct!` / `mdx!` / `#[scene]` - HTML-like UI authoring (`rsx` feature)
- `#[derive(AsAny)]`, `#[derive(BundleEffect)]` and other small helpers
