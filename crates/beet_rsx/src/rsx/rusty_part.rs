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

impl RustyPartMap {
	pub fn collect(node: &mut RsxNode) -> Self {
		let mut visitor = RustyPartVisitor::default();
		visitor.walk_node(node);
		Self(visitor.rusty_map)
	}
}

/// take the effects from a node recursively
#[derive(Default)]
struct RustyPartVisitor {
	rusty_map: HashMap<RustyTracker, RustyPart>,
}

impl RustyPartVisitor {}

impl RsxVisitorMut for RustyPartVisitor {
	fn ignore_block_node_initial(&self) -> bool {
		// we dont want to recurse into initial?
		true
	}

	fn visit_block(&mut self, block: &mut RsxBlock) {
		self.rusty_map
			.insert(block.effect.tracker, RustyPart::RustBlock {
				initial: std::mem::take(&mut block.initial),
				effect: std::mem::take(&mut block.effect),
			});
		// }
	}
	fn visit_attribute(&mut self, attribute: &mut RsxAttribute) {
		match attribute {
			RsxAttribute::Key { .. } => {}
			RsxAttribute::KeyValue { .. } => {}
			RsxAttribute::BlockValue {
				initial, effect, ..
			} => {
				self.rusty_map.insert(
					effect.tracker,
					RustyPart::AttributeValue {
						initial: std::mem::take(initial),
						effect: std::mem::take(effect),
					},
				);
			}
			RsxAttribute::Block { initial, effect } => {
				self.rusty_map.insert(
					effect.tracker,
					RustyPart::AttributeBlock {
						initial: std::mem::take(initial),
						effect: std::mem::take(effect),
					},
				);
			}
		}
	}
	fn visit_component(&mut self, component: &mut RsxComponent) {
		// note how we ignore slot_children, they are handled by RsxTemplateNode
		self.rusty_map
			.insert(component.tracker, RustyPart::Component {
				root: std::mem::take(&mut component.node),
				type_name: std::mem::take(&mut component.type_name),
				ron: std::mem::take(&mut component.ron),
			});
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bar = 2;
		expect(RustyPartMap::collect(&mut rsx! { <div /> }).len()).to_be(0);
		expect(RustyPartMap::collect(&mut rsx! { <div foo=bar /> }).len())
			.to_be(1);
		expect(RustyPartMap::collect(&mut rsx! { <div>{bar}</div> }).len())
			.to_be(1);
	}
}
