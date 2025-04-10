mod signal;
use std::path::Path;

// use crate::rsx::RsxAttribute;
// use crate::rsx::RsxNode;
// use crate::rsx::RsxRust;
use crate::prelude::*;
pub use signal::*;



/// a woefully basic implementation of signals, intended
/// only for testing and as an example implementation for
/// authors of actual reactivity libraries.
/// It aint a segfault, but it's not great.
pub struct Sigfault;

impl Sigfault {
	/// Used by [`RstmlToRsx`] when it encounters a block node:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let block = "hello";
	/// let node = rsx!{<div>{block}</div>};
	/// ```
	pub fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl 'static + Send + Sync + Clone + IntoRsxNode<M>,
	) -> RsxNode {
		RsxNode::Block(RsxBlock {
			initial: Box::new(block.clone().into_node()),
			effect: Effect::new(
				Box::new(move |loc: TreeLocation| {
					effect(move || {
						let block = block.clone();
						DomTarget::with(move |target| {
							let node = block.clone().into_node();
							target.update_rsx_node(node, loc).unwrap()
						});
					});
					Ok(())
				}),
				tracker,
			),
			meta: RsxNodeMeta::default(),
		})
	}

	/// Used by [`RstmlToRsx`] when it encounters an attribute block:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let value = || vec![RsxAttribute::Key{key:"foo".to_string()}];
	/// let node = rsx!{<el {value}/>};
	/// ```
	pub fn parse_attribute_block<M>(
		tracker: RustyTracker,
		block: impl IntoRsxAttributes<M>,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: block.into_rsx_attributes(),
			effect: Effect::new(
				Box::new(|_loc| {
					todo!();
				}),
				tracker,
			),
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
		block: impl 'static + Send + Sync + Clone + IntoSigfaultAttrVal<M>,
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_sigfault_val(),
			effect: Effect::new(
				Box::new(move |loc| {
					effect(move || {
						let value = block.clone().into_sigfault_val();
						println!(
							"would update attribute for {}\n{key}: {value}",
							loc.tree_idx
						);
						todo!();
					});
					Ok(())
				}),
				tracker,
			),
		}
	}
}


pub trait IntoSigfaultAttrVal<M> {
	fn into_sigfault_val(self) -> String;
}

pub struct ToStringIntoSigfaultAttrVal;
impl<T: ToString> IntoSigfaultAttrVal<(T, ToStringIntoSigfaultAttrVal)> for T {
	fn into_sigfault_val(self) -> String { self.to_string() }
}

pub struct FuncIntoSigfaultAttrVal;
impl<T: FnOnce() -> U, U: IntoSigfaultAttrVal<M2>, M2>
	IntoSigfaultAttrVal<(M2, FuncIntoSigfaultAttrVal)> for T
{
	fn into_sigfault_val(self) -> String { self().into_sigfault_val() }
}

pub struct PathIntoSigfaultAttrVal;
impl IntoSigfaultAttrVal<PathIntoSigfaultAttrVal> for &Path {
	fn into_sigfault_val(self) -> String { self.to_string_lossy().to_string() }
}


#[cfg(test)]
mod test {
	use super::signal;
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(7);

		rsx! { <div>value is {get}</div> }
			.xpipe(MountRsDom)
			.unwrap()
			.xpipe(RegisterEffects::default())
			.unwrap();
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 7</div>");
		set(8);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 8</div>");
		set(9);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 9</div>");
	}
	#[test]
	fn components() {
		let (get, set) = signal(7);

		rsx! { <div>value is {get}</div> }
			.xpipe(MountRsDom)
			.unwrap()
			.xpipe(RegisterEffects::default())
			.unwrap();
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 7</div>");
		set(8);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 8</div>");
		set(9);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"1\">value is 9</div>");
	}
}
