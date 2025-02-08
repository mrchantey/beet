use crate::prelude::*;
use std::collections::HashMap;


impl std::ops::Deref for RsxHydratedMap {
	type Target = HashMap<RustyTracker, RsxHydratedNode>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for RsxHydratedMap {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}



pub enum RsxHydratedNode {
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

impl std::fmt::Debug for RsxHydratedNode {
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




pub struct RsxHydratedMap(pub HashMap<RustyTracker, RsxHydratedNode>);

impl RsxHydratedMap {
	pub fn collect(node: impl Rsx) -> ParseResult<Self> {
		let mut visitor = RsxHydratedVisitor::default();
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
struct RsxHydratedVisitor {
	rusty_map: HashMap<RustyTracker, RsxHydratedNode>,
	err: Option<ParseError>,
}

impl RsxHydratedVisitor {
	fn take_effect(
		&mut self,
		effect: &mut Effect,
	) -> Option<(RegisterEffect, RustyTracker)> {
		let effect = effect.take();
		let tracker = effect.tracker.ok_or_else(|| {
			ParseError::Hydration(format!("effect has no tracker, this can happen if collect tracker was disabled or they were already collected"))
		});
		match tracker {
			Err(err) => {
				self.err = Some(err);
				return None;
			}
			Ok(tracker) => Some((effect.register, tracker)),
		}
	}
}

impl RsxVisitorMut for RsxHydratedVisitor {
	fn ignore_block_node_initial(&self) -> bool {
		// we dont want to recurse into initial?
		true
	}

	fn visit_block(&mut self, block: &mut RsxBlock) {
		if let Some((register, tracker)) = self.take_effect(&mut block.effect) {
			self.rusty_map.insert(tracker, RsxHydratedNode::RustBlock {
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
					self.rusty_map.insert(
						tracker,
						RsxHydratedNode::AttributeValue {
							initial: std::mem::take(initial),
							register,
						},
					);
				}
			}
			RsxAttribute::Block { initial, effect } => {
				if let Some((register, tracker)) = self.take_effect(effect) {
					self.rusty_map.insert(
						tracker,
						RsxHydratedNode::AttributeBlock {
							initial: std::mem::take(initial),
							register,
						},
					);
				}
			}
		}
	}
	fn visit_component(&mut self, component: &mut RsxComponent) {
		match std::mem::take(&mut component.tracker) {
			Some(tracker) => {
				// note how we ignore slot_children, they are handled by RsxTemplateNode
				self.rusty_map.insert(tracker, RsxHydratedNode::Component {
					root: std::mem::take(&mut component.root),
				});
			}
			None => {
				self.err = Some(ParseError::Hydration(
					"component has no tracker, this can happen if collect tracker was disabled or they were already collected".into(),
				));
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
		expect(RsxHydratedMap::collect(rsx! { <div /> }).unwrap().len())
			.to_be(0);
		expect(
			RsxHydratedMap::collect(rsx! { <div foo=bar /> })
				.unwrap()
				.len(),
		)
		.to_be(1);
		expect(
			RsxHydratedMap::collect(rsx! { <div>{bar}</div> })
				.unwrap()
				.len(),
		)
		.to_be(1);
	}
}
