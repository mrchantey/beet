use crate::prelude::*;

/// A serializable counterpart to a [`RustyPart`]
/// This struct performs two roles:
/// 1. hydration splitting and joining
/// 2. storing the hash of a rusty part token stream, for hot reload diffing
///
/// The combination of an index and tokens hash guarantees the level of
/// diffing required to detect when a recompile is necessary.
/// ```rust ignore
/// let tree = rsx!{<div {rusty} key=73 key=rusty key={rusty}>other text{rusty}more text <Component key=value/></div>}
/// //							      ^^^^^             ^^^^^      ^^^^^             ^^^^^            ^^^^^^^^^^^^^^^^^^^
/// //							      attr blocks       idents     value blocks      node blocks      Component open tags
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RustyTracker {
	/// the order in which this part was visited by the syn::Visitor
	pub index: u32,
	/// a hash of the token stream for this part
	pub tokens_hash: u64,
}


impl RustyTracker {
	pub const PLACEHOLDER: Self = Self {
		index: u32::MAX,
		tokens_hash: u64::MAX,
	};

	pub fn new(index: u32, tokens_hash: u64) -> Self {
		Self { index, tokens_hash }
	}
	/// sometimes we want to diff a tree without the trackers
	pub fn clear(&mut self) {
		self.index = 0;
		self.tokens_hash = 0;
	}
}


/// The parts of an rsx! macro that are not serializable are
/// called Rusty Parts.
#[derive(Debug)]
pub enum RustyPart {
	// we also collect components because they
	// cannot be statically resolved
	Component {
		root: RsxNode,
		/// Type names cannot be discovered statically
		type_name: String,
		/// for client islands, cannot be statically created
		ron: Option<String>,
	},
	RustBlock {
		initial: RsxNode,
		effect: Effect,
	},
	AttributeBlock {
		initial: Vec<RsxAttribute>,
		effect: Effect,
	},
	AttributeValue {
		initial: String,
		effect: Effect,
	},
}


#[derive(Deref, DerefMut)]
pub struct RustyPartMap(pub HashMap<RustyTracker, RustyPart>);

pub struct NodeToRustyPartMap;

impl Pipeline<RsxNode, RustyPartMap> for NodeToRustyPartMap {
	fn apply(self, mut node: RsxNode) -> RustyPartMap {
		let mut rusty_map = HashMap::default();
		VisitRsxNodeMut::walk_with_opts(
			&mut node,
			// we dont want to recurse into initial?
			VisitRsxOptions::ignore_block_node_initial(),
			|node| match node {
				RsxNode::Block(block) => {
					rusty_map.insert(
						block.effect.tracker,
						RustyPart::RustBlock {
							initial: std::mem::take(&mut block.initial),
							effect: std::mem::take(&mut block.effect),
						},
					);
				}
				RsxNode::Element(el) => {
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
				RsxNode::Component(component) => {
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
