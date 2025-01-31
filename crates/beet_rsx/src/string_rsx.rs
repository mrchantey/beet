use crate::prelude::*;
use crate::rsx::IntoRsx;
use crate::rsx::RsxAttribute;

/// A simple non-reactive rsx implementation
#[derive(Debug)]
pub struct StringRsx;


impl StringRsx {
	pub fn map_node_block<M>(
		block: impl 'static + Clone + IntoRsx<M>,
	) -> RsxNode {
		RsxNode::Block {
			initial: Box::new(block.clone().into_rsx()),
			register_effect: Box::new(move |_| {}),
		}
	}
	pub fn map_attribute_block(
		mut block: impl 'static + FnMut() -> RsxAttribute,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: vec![block()],
			register_effect: Box::new(move |_| {}),
		}
	}
	pub fn map_attribute_value<M>(
		key: &str,
		block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RsxAttribute {
		let key = key.to_string();
		RsxAttribute::KeyValue {
			key: key.clone(),
			value: block.clone().into_attribute_value(),
		}
	}
}
