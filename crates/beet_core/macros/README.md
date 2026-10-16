# beet_core_macros

Procedural macros for `beet_core`.

These are re-exported through `beet_core` (and in turn `beet`), so depend on those rather than this crate directly. It provides:

- `#[action]` - turn a function into a callable [`Action`] (see `beet_action`), its `#[field]` parameters struct fields and its `#[param]` and `Params<T>` parameters request params it declares by taking
- `#[derive(Get, Set, SetWith)]` - per-field getters and setters
- `#[derive(StringPrimitive)]` - the conversions a validated string newtype over `SmolStr` shares, through its own `parse`
- `#[derive(ToTokens)]` - tokenize a value back into its constructing source
- `#[main]` / `#[test]` - async entry point and the beet test attribute
- `rsx!` / `mdx!` / `#[template]` - HTML-like UI authoring (`rsx` feature)
- `#[derive(AsAny)]`, `#[derive(BundleEffect)]` and other small helpers
