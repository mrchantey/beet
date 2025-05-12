#[cfg(feature = "reactive_graph")]
mod reactive_graph_runtime;
#[cfg(feature = "reactive_graph")]
pub use reactive_graph_runtime::*;
pub mod sigfault;
mod sigfault_runtime;
mod string_runtime;
use crate::prelude::*;
pub use sigfault_runtime::*;
use std::path::Path;
pub use string_runtime::*;
// pub use reactive_graph_runtime::*;

use beet_common::prelude::*;

/// An rsx Runtime handles creation of reactive web nodes,
/// often by  
pub trait Runtime {
	/// A type which all attribute values must be able to be converted to,
	/// see [`IntoAttrVal`].
	type AttributeValue;

	/// Used by [`RstmlToRsx`] when it encounters a block node:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let block = "hello";
	/// let node = rsx!{<div>{block}</div>};
	/// ```
	fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl 'static + Send + Sync + Clone + IntoWebNode<M>,
	) -> WebNode;
	/// Used by [`RstmlToRsx`] when it encounters an attribute block:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// #[derive(IntoBlockAttribute)]
	/// struct Foo;
	/// let node = rsx!{<el {Foo}/>};
	/// ```
	fn parse_attribute_block<M>(
		tracker: RustyTracker,
		block: impl IntoBlockAttribute<M>,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: block.initial_attributes(),
			effect: Effect::new(
				Box::new(move |loc| block.register_effects(loc)),
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
	fn parse_attribute_value<M>(
		key: &'static str,
		tracker: RustyTracker,
		block: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoAttrVal<Self::AttributeValue, M>,
	) -> RsxAttribute;
	/// Called by both `parse_attribute_value` and the implementation of
	/// `parse_attribute_block` where the block contains non-event fields.
	/// Note that in the case of an attribute block `<foo {bar}/>` all
	/// attributes are registered, even the static ones.
	fn register_attribute_effect<M>(
		loc: TreeLocation,
		key: &'static str,
		block: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoAttrVal<Self::AttributeValue, M>,
	);
}




pub trait IntoAttrVal<T, M> {
	fn into_val(self) -> T;
}

pub struct ToStringIntoAttrVal;
impl<T: ToString> IntoAttrVal<String, ToStringIntoAttrVal> for T {
	fn into_val(self) -> String { self.to_string() }
}

/// Implement any function that returns an [`IntoAttrVal`] type
pub struct FuncIntoAttrVal;
impl<T: FnOnce() -> U, U: IntoAttrVal<U, M2>, M2>
	IntoAttrVal<U, (M2, FuncIntoAttrVal)> for T
{
	fn into_val(self) -> U { self().into_val() }
}

pub struct PathIntoAttrVal;
impl IntoAttrVal<String, PathIntoAttrVal> for &Path {
	fn into_val(self) -> String { self.to_string_lossy().to_string() }
}
