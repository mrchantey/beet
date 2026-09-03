use beet_core::prelude::*;

/// Declares where an entry's repo store begins: `src` names a position relative
/// to the entry document's own location *in its store*, so its
/// `<RoutesDir>`/`<TemplateDir>`/`<AssetsDir>` mounts can reach paths outside
/// the entry's own directory.
///
/// The declaration half of the repo store, and the only half an entry document
/// carries: `RepoRoot` says where the store should be rooted, `RepoStore` marks
/// the store entry resolution actually built on the root.
///
/// Rebasing is one store operation (`BlobStore::rebase_repo`), uniform across
/// kinds: a filesystem store re-roots at the resolved directory (walking above
/// the entry's directory is the point), a self-rooted store (a bucket, browser
/// storage) takes a key-prefix view for a root at or under its own and fails
/// loudly when the root escapes the store — catching a store published from
/// the entry's directory instead of its declared universe. Eg an entry at
/// `examples/wasm/main.bsx` declaring:
///
/// ```html
/// <RepoRoot src="../.."/>
/// <AssetsDir src="assets" prefix="assets"/>
/// ```
///
/// roots the store at the workspace (or a bucket published from it at the
/// bucket root), replacing the old `--root` cli flag: the entry owns its root
/// rather than every caller re-supplying it. Inert at runtime; only entry
/// resolution reads it, through [`EntryPrescan`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RepoRoot {
	/// The repo store root, relative to the entry document's location in its store.
	pub src: String,
}
