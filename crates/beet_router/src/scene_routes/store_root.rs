use beet_core::prelude::*;

/// Widens an entry document's store root to an ancestor directory, so its
/// `<RoutesDir>`/`<TemplateDir>`/`<AssetsDir>` mounts can reach paths outside
/// the entry's own directory.
///
/// `src` is relative to the entry document's directory. The `beet` binary
/// pre-scans the raw entry for this declaration before building the store, so
/// every `src` in the document resolves against the widened root, eg an entry
/// at `examples/wasm/main.bsx` declaring:
///
/// ```html
/// <StoreRoot src="../.."/>
/// <AssetsDir src="assets" prefix="assets"/>
/// ```
///
/// roots the store at the workspace, replacing the old `--root` cli flag: the
/// entry owns its root rather than every caller re-supplying it. Inert at
/// runtime; only entry resolution reads it, through [`EntryPrescan`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct StoreRoot {
	/// The store root directory, relative to the entry document's directory.
	pub src: String,
}
