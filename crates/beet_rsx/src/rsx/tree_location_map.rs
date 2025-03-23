use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap;



pub struct NodeToTreeLocationMap;

impl<T: RsxPipelineTarget + AsRef<RsxNode>> RsxPipeline<T, TreeLocationMap>
	for NodeToTreeLocationMap
{
	fn apply(self, node: T) -> TreeLocationMap {
		let mut map = TreeLocationMap::default();

		TreeLocationVisitor::visit(node.as_ref(), |loc, node| {
			match node {
				RsxNode::Block(_) => {
					map.rusty_locations.insert(loc.tree_idx, loc);
				}
				RsxNode::Element(el) => {
					// println!("el loc: {}", loc.tree_idx);
					if el.children.directly_contains_rust_node() {
						let encoded =
							TextBlockEncoder::encode(loc.tree_idx, el);
						map.collapsed_elements.insert(loc.tree_idx, encoded);
					}
				}
				_ => {}
			}
		});
		map
	}
}



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

impl RsxPipelineTarget for TreeLocationMap {}


impl TreeLocationMap {
	/// a best-effort check for validity of a tree location map
	pub fn check_valid(&self, node: &RsxNode) -> Result<()> {
		let mut idx_incr = TreeIdxIncr::default();

		let mut result = Ok(());

		VisitRsxNode::walk(node, |node| {
			let tree_idx = idx_incr.next();

			if let Some(_) = self.collapsed_elements.get(&tree_idx) {
				if let RsxNode::Element(_) = node {
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
		let map = (&root.node).pipe(NodeToTreeLocationMap);

		map.check_valid(&root.node).unwrap();


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


	#[test]
	fn consequtive_collapsed_nodes() {
		use beet::prelude::*;

		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> RsxRoot {
			let val = 4;
			rsx! { <div>{val}</div> }
		}


		let root = rsx! {
			<MyComponent />
			<MyComponent />
		}
		.pipe(SlotsPipeline::default())
		.unwrap();

		let html = (&root)
			.pipe(RsxToHtml::default())
			.pipe(RenderHtml::default())
			.unwrap();
		expect(html).to_be(
			"<div data-beet-rsx-idx=\"3\">4</div><div data-beet-rsx-idx=\"8\">4</div>",
		);

		let map = (&root.node).pipe(NodeToTreeLocationMap);
		expect(map.collapsed_elements.get(&TreeIdx::new(3))).to_be_some();
		expect(map.collapsed_elements.get(&TreeIdx::new(8))).to_be_some();
	}
}
