use crate::prelude::*;
use crate::rsx::IntoRsxNode;
use crate::rsx::RsxAttribute;

/// A simple non-reactive rsx runtime
#[derive(Debug)]
pub struct StringRuntime;

fn noop() -> RegisterEffect { Box::new(|_| Ok(())) }
impl Runtime for StringRuntime {
	type AttributeValue = String;
	fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl IntoRsxNode<M>,
	) -> RsxNode {
		RsxNode::Block(RsxBlock {
			initial: Box::new(block.into_node()),
			effect: Effect::new(noop(), tracker),
			meta: NodeMeta::default(),
		})
	}

	fn parse_attribute_block<M>(
		tracker: RustyTracker,
		block: impl IntoBlockAttribute<M>,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: block.initial_attributes(),
			effect: Effect::new(noop(), tracker),
		}
	}

	fn parse_attribute_value<M>(
		key: &'static str,
		tracker: RustyTracker,
		block: impl 'static + Clone + IntoAttrVal<Self::AttributeValue, M>,
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_val(),
			effect: Effect::new(noop(), tracker),
		}
	}
	fn register_attribute_effect<M>(
		_loc: TreeLocation,
		_key: &'static str,
		_block: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoAttrVal<Self::AttributeValue, M>,
	) {
		// noop
	}
}
