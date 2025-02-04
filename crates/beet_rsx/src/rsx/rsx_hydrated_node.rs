use crate::prelude::*;
use std::collections::HashMap;


pub enum RsxHydratedNode {
	// we also collect components because they
	// cannot be statically resolved
	Component {
		node: RsxNode,
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
			Self::Component { node } => {
				f.debug_struct("Component").field("node", node).finish()
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

impl RsxHydratedNode {
	pub fn collect(node: impl Rsx) -> HashMap<LineColumn, Self> {
		let mut effects = HashMap::new();

		let take_effect = |effect: &mut Effect| {
			let effect = effect.take();
			let location = effect
				.location
				.expect("effect has no location, ensure they are collected");
			(effect.register, location)
		};

		node.into_rsx().visit_mut(|node| match node {
			RsxNode::Block { effect, initial } => {
				let (register, location) = take_effect(effect);
				effects.insert(location, Self::RustBlock {
					initial: std::mem::take(initial),
					register,
				});
			}
			RsxNode::Element(rsx_element) => {
				for attr in rsx_element.attributes.iter_mut() {
					match attr {
						RsxAttribute::Key { .. } => {}
						RsxAttribute::KeyValue { .. } => {}
						RsxAttribute::BlockValue {
							initial, effect, ..
						} => {
							let (register, location) = take_effect(effect);
							effects.insert(location, Self::AttributeValue {
								initial: std::mem::take(initial),
								register,
							});
						}
						RsxAttribute::Block { initial, effect } => {
							let (register, location) = take_effect(effect);
							effects.insert(location, Self::AttributeBlock {
								initial: std::mem::take(initial),
								register,
							});
						}
					}
				}
			}
			RsxNode::Component { loc, node, .. } => {
				let loc = std::mem::take(loc).expect(
					"component has no location, ensure they are collected",
				);
				effects.insert(loc, Self::Component {
					node: std::mem::take(node),
				});
			}
			_ => {}
		});
		effects
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bar = 2;
		expect(RsxHydratedNode::collect(rsx! { <div /> }).len()).to_be(0);
		expect(RsxHydratedNode::collect(rsx! { <div foo=bar /> }).len()).to_be(1);
		expect(RsxHydratedNode::collect(rsx! { <div>{bar}</div> }).len())
			.to_be(1);
	}
}
