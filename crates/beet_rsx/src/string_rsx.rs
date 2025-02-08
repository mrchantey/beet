use crate::prelude::*;
use crate::rsx::IntoRsx;
use crate::rsx::RsxAttribute;

/// A simple non-reactive rsx implementation
#[derive(Debug)]
pub struct StringRsx;

fn noop() -> RegisterEffect { Box::new(|_| Ok(())) }
impl StringRsx {
	pub fn register_block<M>(
		_block: impl 'static + Clone + IntoRsx<M>,
	) -> RegisterEffect {
		noop()
	}
	pub fn register_attribute_block(
		_block: impl 'static + FnMut() -> RsxAttribute,
	) -> RegisterEffect {
		noop()
	}
	pub fn register_attribute_value<M>(
		_key: &str,
		_block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RegisterEffect {
		noop()
	}
}
