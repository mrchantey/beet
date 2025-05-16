use super::sigfault::*;
use crate::prelude::*;
use beet_common::prelude::*;

/// a woefully basic implementation of signals, intended
/// only for testing and as an example implementation for
/// authors of actual reactivity libraries.
/// It aint a segfault, but it's not great.
pub struct SigfaultRuntime;

impl Runtime for SigfaultRuntime {
	type AttributeValue = String;

	fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl 'static + Send + Sync + Clone + IntoWebNode<M>,
	) -> WebNode {
		WebNode::Block(BlockNode {
			initial: Box::new(block.clone().into_node()),
			effect: Effect::new(
				Box::new(move |loc: TreeLocation| {
					effect(move || {
						let block = block.clone();
						DomTarget::with(move |target| {
							let node = block.clone().into_node();
							target.update_web_node(loc, node).unwrap()
						});
					});
					Ok(())
				}),
				tracker,
			),
			meta: NodeMeta::default(),
		})
	}

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
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_val(),
			effect: Effect::new(
				Box::new(move |loc| {
					Self::register_attribute_effect(loc, key, block);
					Ok(())
				}),
				tracker,
			),
		}
	}

	fn register_attribute_effect<M>(
		loc: TreeLocation,
		key: &'static str,
		block: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoAttrVal<Self::AttributeValue, M>,
	) {
		effect(move || {
			let value = block.clone().into_val();
			DomTarget::with(move |target| {
				target.update_rsx_attribute(loc, key, &value).unwrap()
			});
		});
	}
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
			.xpipe(MountToRsDom)
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
			.xpipe(MountToRsDom)
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
