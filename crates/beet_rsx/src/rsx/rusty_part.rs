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
pub enum RustyPart {
	// we also collect components because they
	// cannot be statically resolved
	Component {
		root: RsxRoot,
	},
	RustBlock {
		initial: RsxNode,
		register: RegisterEffect,
	},
	AttributeBlock {
		initial: Vec<RsxAttribute>,
		register: RegisterEffect,
	},
	AttributeValue {
		initial: String,
		register: RegisterEffect,
	},
}

impl std::fmt::Debug for RustyPart {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Component { root } => {
				f.debug_struct("Component").field("root", root).finish()
			}
			Self::RustBlock { initial, register } => f
				.debug_struct("RustBlock")
				.field("initial", initial)
				.field("register", &std::any::type_name_of_val(&register))
				.finish(),
			Self::AttributeBlock { initial, register } => f
				.debug_struct("AttributeBlock")
				.field("initial", initial)
				.field("register", &std::any::type_name_of_val(&register))
				.finish(),
			Self::AttributeValue { initial, register } => f
				.debug_struct("AttributeValue")
				.field("initial", initial)
				.field("register", &std::any::type_name_of_val(&register))
				.finish(),
		}
	}
}


#[derive(Deref, DerefMut)]
pub struct RustyPartMap(pub HashMap<RustyTracker, RustyPart>);

impl RustyPartMap {
	pub fn collect(node: impl Rsx) -> TemplateResult<Self> {
		let mut visitor = RustyPartVisitor::default();
		let mut node = node.into_rsx();
		visitor.walk_node(&mut node);
		if let Some(err) = visitor.err {
			Err(err)
		} else {
			Ok(Self(visitor.rusty_map))
		}
	}
}

/// take the effects from a node recursively
#[derive(Default)]
struct RustyPartVisitor {
	rusty_map: HashMap<RustyTracker, RustyPart>,
	err: Option<TemplateError>,
}

impl RustyPartVisitor {
	fn take_effect(
		&mut self,
		effect: &mut Effect,
	) -> Option<(RegisterEffect, RustyTracker)> {
		let effect = effect.take();
		let tracker = effect
			.tracker
			.ok_or_else(|| TemplateError::NoRustyPart("Effect"));
		match tracker {
			Err(err) => {
				self.err = Some(err);
				return None;
			}
			Ok(tracker) => Some((effect.register, tracker)),
		}
	}
}

impl RsxVisitorMut for RustyPartVisitor {
	fn ignore_block_node_initial(&self) -> bool {
		// we dont want to recurse into initial?
		true
	}

	fn visit_block(&mut self, block: &mut RsxBlock) {
		if let Some((register, tracker)) = self.take_effect(&mut block.effect) {
			self.rusty_map.insert(tracker, RustyPart::RustBlock {
				initial: std::mem::take(&mut block.initial),
				register,
			});
		}
	}
	fn visit_attribute(&mut self, attribute: &mut RsxAttribute) {
		match attribute {
			RsxAttribute::Key { .. } => {}
			RsxAttribute::KeyValue { .. } => {}
			RsxAttribute::BlockValue {
				initial, effect, ..
			} => {
				if let Some((register, tracker)) = self.take_effect(effect) {
					self.rusty_map.insert(tracker, RustyPart::AttributeValue {
						initial: std::mem::take(initial),
						register,
					});
				}
			}
			RsxAttribute::Block { initial, effect } => {
				if let Some((register, tracker)) = self.take_effect(effect) {
					self.rusty_map.insert(tracker, RustyPart::AttributeBlock {
						initial: std::mem::take(initial),
						register,
					});
				}
			}
		}
	}
	fn visit_component(&mut self, component: &mut RsxComponent) {
		match std::mem::take(&mut component.tracker) {
			Some(tracker) => {
				// note how we ignore slot_children, they are handled by RsxTemplateNode
				self.rusty_map.insert(tracker, RustyPart::Component {
					root: std::mem::take(&mut component.root),
				});
			}
			None => {
				self.err = Some(TemplateError::NoRustyPart("Component"));
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bar = 2;
		expect(RustyPartMap::collect(rsx! { <div /> }).unwrap().len()).to_be(0);
		expect(
			RustyPartMap::collect(rsx! { <div foo=bar /> })
				.unwrap()
				.len(),
		)
		.to_be(1);
		expect(
			RustyPartMap::collect(rsx! { <div>{bar}</div> })
				.unwrap()
				.len(),
		)
		.to_be(1);
	}
}
