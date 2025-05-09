use beet_common::prelude::*;
use crate::prelude::*;

/// The parts of an rsx! macro that are not serializable are
/// called Rusty Parts.
#[derive(Debug)]
pub enum RustyPart {
	// we also collect components because they
	// cannot be statically resolved
	Component {
		root: WebNode,
		/// Type names cannot be discovered statically
		type_name: String,
		/// for client islands, cannot be statically created
		ron: Option<String>,
	},
	RustBlock {
		initial: WebNode,
		effect: Effect,
	},
	AttributeValue {
		/// The initial valu cannot be statically created
		initial: String,
		effect: Effect,
	},
	AttributeBlock {
		/// The initial valu cannot be statically created
		initial: Vec<(String, Option<String>)>,
		effect: Effect,
	},
}


#[derive(Deref, DerefMut)]
pub struct RustyPartMap(pub HashMap<RustyTracker, RustyPart>);

pub struct NodeToRustyPartMap;

impl Pipeline<WebNode, RustyPartMap> for NodeToRustyPartMap {
	fn apply(self, mut node: WebNode) -> RustyPartMap {
		let mut rusty_map = HashMap::default();
		VisitWebNodeMut::walk_with_opts(
			&mut node,
			// we dont want to recurse into initial?
			VisitRsxOptions::ignore_block_node_initial(),
			|node| match node {
				WebNode::Block(block) => {
					rusty_map.insert(
						block.effect.tracker,
						RustyPart::RustBlock {
							initial: std::mem::take(&mut block.initial),
							effect: std::mem::take(&mut block.effect),
						},
					);
				}
				WebNode::Element(el) => {
					for attr in el.attributes.iter_mut() {
						match attr {
							RsxAttribute::Key { .. } => {}
							RsxAttribute::KeyValue { .. } => {}
							RsxAttribute::BlockValue {
								initial,
								effect,
								..
							} => {
								rusty_map.insert(
									effect.tracker,
									RustyPart::AttributeValue {
										initial: std::mem::take(initial),
										effect: std::mem::take(effect),
									},
								);
							}
							RsxAttribute::Block { initial, effect } => {
								rusty_map.insert(
									effect.tracker,
									RustyPart::AttributeBlock {
										initial: std::mem::take(initial),
										effect: std::mem::take(effect),
									},
								);
							}
						}
					}
				}
				WebNode::Component(component) => {
					// note how we ignore slot_children, they are handled by RsxTemplateNode
					rusty_map.insert(component.tracker, RustyPart::Component {
						root: std::mem::take(&mut component.node),
						type_name: std::mem::take(&mut component.type_name),
						ron: std::mem::take(&mut component.ron),
					});
				}
				_ => {}
			},
		);
		RustyPartMap(rusty_map)
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bar = 2;
		expect(rsx! { <div /> }.xpipe(NodeToRustyPartMap).len()).to_be(0);
		expect(rsx! { <div foo=bar /> }.xpipe(NodeToRustyPartMap).len())
			.to_be(1);
		expect(rsx! { <div>{bar}</div> }.xpipe(NodeToRustyPartMap).len())
			.to_be(1);
	}
}
