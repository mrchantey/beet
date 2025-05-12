use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use rapidhash::RapidHashMap;



pub struct NodeToTreeLocationMap;

impl<T: AsRef<WebNode>> Pipeline<T, TreeLocationMap> for NodeToTreeLocationMap {
	fn apply(self, node: T) -> TreeLocationMap {
		let mut map = TreeLocationMap::default();

		TreeLocationVisitor::visit(node.as_ref(), |loc, node| {
			match node {
				WebNode::Block(RsxBlock { effect, .. }) => {
					map.rusty_locations.insert(effect.tracker, loc);
				}
				WebNode::Element(el) => {
					if el.children.directly_contains_rust_node() {
						let encoded =
							TextBlockEncoder::encode(loc.tree_idx, el);
						map.collapsed_elements.insert(loc.tree_idx, encoded);
					}
				}
				WebNode::Component(comp) => {
					map.rusty_locations.insert(comp.tracker, loc);
				}
				_ => {}
			}
		});
		map
	}
}


/// One of the essential components of resumability, allowing us to map
/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeLocationMap {
	/// Used to resolve the location of a rusty part by its tracker
	pub rusty_locations: RapidHashMap<RustyTracker, TreeLocation>,
	pub collapsed_elements: RapidHashMap<TreeIdx, TextBlockEncoder>,
}


impl TreeLocationMap {
	/// a best-effort check for validity of a tree location map
	pub fn check_valid(&self, node: &WebNode) -> Result<()> {
		let mut idx_incr = TreeIdxIncr::default();

		let mut result = Ok(());

		VisitWebNode::walk(node, |node| {
			let tree_idx = idx_incr.next();

			if let Some(_) = self.collapsed_elements.get(&tree_idx) {
				if let WebNode::Element(_) = node {
				} else {
					result = Err(anyhow::anyhow!(
						"parent element {tree_idx} does not exist for text block encoder"
					));
				}
			}
		});
		Ok(())
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
		let map = (&root).xpipe(NodeToTreeLocationMap);

		map.check_valid(&root).unwrap();

		expect(map.collapsed_elements).to_be(
			vec![(1.into(), TextBlockEncoder {
				parent_id: 1.into(),
				split_positions: vec![vec![4, 5, 5], vec![10, 9]],
			})]
			.into_iter()
			.collect::<HashMap<_, _>>(),
		);
		let mut locations = map.rusty_locations.iter().collect::<Vec<_>>();
		locations.sort_by(|a, b| a.0.index.cmp(&b.0.index));
		// {desc}
		expect(locations[0].1).to_be(&TreeLocation::new(4, 1, 1));
		// {color}
		expect(locations[1].1).to_be(&TreeLocation::new(7, 1, 3));
		// {action}
		expect(locations[2].1).to_be(&TreeLocation::new(11, 1, 5));
	}


	#[test]
	fn consequtive_collapsed_nodes() {
		use beet::prelude::*;

		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> WebNode {
			let val = 4;
			rsx! { <div>{val}</div> }
		}


		let node = rsx! {
			<MyComponent />
			<MyComponent />
		}
		.xpipe(ApplySlots::default())
		.unwrap();

		let html = node
			.xref()
			.xpipe(RsxToHtml::default())
			.xpipe(RenderHtml::default());
		expect(html).to_be(
			"<div data-beet-rsx-idx=\"3\">4</div><div data-beet-rsx-idx=\"8\">4</div>",
		);

		let map = node.xpipe(NodeToTreeLocationMap);
		expect(map.collapsed_elements.get(&TreeIdx::new(3))).to_be_some();
		expect(map.collapsed_elements.get(&TreeIdx::new(8))).to_be_some();
	}
}
