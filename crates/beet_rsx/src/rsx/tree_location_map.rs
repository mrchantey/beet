use rapidhash::RapidHashMap;

use crate::prelude::*;

/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeLocationMap {
	// we could technically use a vec where the indices are 'block_idx',
	// and track block_idx in the [TreeLocation]
	// but at this stage of the project thats harder to reason about
	// and this provides symmetry with [Self::collapsed_elements]
	pub rusty_locations: RapidHashMap<TreeIdx, TreeLocation>,
	pub collapsed_elements: RapidHashMap<TreeIdx, TextBlockEncoder>,
}

impl TreeLocationMap {
	// TODO pipeline
	pub fn from_node(node: &RsxNode) -> Self {
		let mut map = Self::default();

		TreeLocationVisitor::visit(node, |loc, node| match node {
			RsxNode::Block(_) => {
				map.rusty_locations.insert(loc.tree_idx, loc);
			}
			RsxNode::Element(el) => {
				if el.children.directly_contains_rust_node() {
					let encoded = TextBlockEncoder::encode(loc.tree_idx, el);
					map.collapsed_elements.insert(loc.tree_idx, encoded);
				}
			}
			_ => {}
		});
		map
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let desc = "quick";
		let color = "brown";
		let action = "jumps over";

		let root = rsx! { <div>"The "{desc}" and "{color}<b>fox</b>{action}the lazy " dog"</div> };

		let map = TreeLocationMap::from_node(&root);

		expect(map.collapsed_elements).to_be(
			vec![(1.into(), TextBlockEncoder {
				parent_id: 1.into(),
				split_positions: vec![vec![4, 5, 5], vec![10, 9]],
			})]
			.into_iter()
			.collect::<HashMap<_, _>>(),
		);
		// {desc}
		expect(&map.rusty_locations[&4.into()])
			.to_be(&TreeLocation::new(4, 1, 1));
		// {color}
		expect(&map.rusty_locations[&7.into()])
			.to_be(&TreeLocation::new(7, 1, 3));
		// {action}
		expect(&map.rusty_locations[&11.into()])
			.to_be(&TreeLocation::new(11, 1, 5));
	}
}
