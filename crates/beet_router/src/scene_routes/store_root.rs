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
/// runtime; only entry resolution reads it.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct StoreRoot {
	/// The store root directory, relative to the entry document's directory.
	pub src: String,
}

impl StoreRoot {
	/// The `src` of the first `<StoreRoot>` element in a parsed entry tree, the
	/// registry-free pre-scan entry resolution runs before building the store.
	pub fn extract_root(nodes: &[BsxNode]) -> Option<String> {
		nodes.iter().find_map(|node| match node {
			BsxNode::Element(element) if element.tag == "StoreRoot" => element
				.attributes
				.iter()
				.find(|attr| attr.key == "src")
				.and_then(|attr| match &attr.value {
					AttrValue::Str(src) => Some(src.clone()),
					_ => None,
				}),
			BsxNode::Element(element) => Self::extract_root(&element.children),
			_ => None,
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn extracts_the_first_store_root() {
		let nodes = parse_document(
			"<Router><StoreRoot src=\"../..\"/><TemplateDir src=\"templates\"/></Router>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		StoreRoot::extract_root(&nodes).xpect_eq(Some("../..".to_string()));
		let none = parse_document("<Router/>", &BsxParseConfig::bsx()).unwrap();
		StoreRoot::extract_root(&none).xpect_none();
	}
}
