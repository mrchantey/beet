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

	/// Used by [`RstmlToRsx`] when it encounters an attribute with a block value:
	/// ```
	/// # use beet_rsx::prelude::*;
	/// let value = 3;
	/// let node = rsx!{<el key={value}/>};
	/// ```
	pub fn parse_attribute_value<M>(
		key: &'static str,
		tracker: RustyTracker,
		block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_attribute_value(),
			effect: Effect::new(noop(), tracker),
		}
	}
}
