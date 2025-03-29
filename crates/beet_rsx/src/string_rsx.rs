use crate::prelude::*;
use crate::rsx::IntoRsxRoot;
use crate::rsx::RsxAttribute;
use crate::sigfault::IntoSigfaultAttrVal;

/// A simple non-reactive rsx runtime
#[derive(Debug)]
pub struct StringRsx;

fn noop() -> RegisterEffect { Box::new(|_| Ok(())) }
impl StringRsx {
	/// Used by [`RstmlToRsx`] when it encounters a block node:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let block = "hello";
	/// let node = rsx!{<div>{block}</div>};
	/// ```
	pub fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl IntoRsxRoot<M>,
	) -> RsxNode {
		RsxNode::Block(RsxBlock {
			initial: Box::new(block.into_root()),
			effect: Effect::new(noop(), tracker),
		})
	}

	/// Used by [`RstmlToRsx`] when it encounters an attribute block:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let value = vec![RsxAttribute::Key{key:"foo".to_string()}];
	/// let node = rsx!{<el {value}/>};
	/// ```
	pub fn parse_attribute_block<M>(
		tracker: RustyTracker,
		block: impl IntoRsxAttributes<M>,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: block.into_rsx_attributes(),
			effect: Effect::new(noop(), tracker),
		}
	}


	/// Used by [`RstmlToRsx`] when it encounters an attribute with a block value:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let value = 3;
	/// let node = rsx!{<el key={value}/>};
	/// ```
	pub fn parse_attribute_value<M>(
		key: &'static str,
		tracker: RustyTracker,
		block: impl 'static + Clone + IntoSigfaultAttrVal<M>,
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_sigfault_val(),
			effect: Effect::new(noop(), tracker),
		}
	}
}
