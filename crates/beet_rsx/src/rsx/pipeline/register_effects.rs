use crate::prelude::*;

use anyhow::Result;

/// Registers all effects for a node and its children
#[derive(Default)]
pub struct RegisterEffects {
	/// The initial location used by the [`TreeLocationVisitor`]
	pub root_location: TreeLocation,
}

impl RegisterEffects {
	/// Create a new instance with a root location
	pub fn new(root_location: TreeLocation) -> Self { Self { root_location } }
}

impl<T: Into<WebNode>> Pipeline<T, Result<()>> for RegisterEffects {
	fn apply(self, node: T) -> Result<()> {
		let mut node: WebNode = node.into();
		let mut result = Ok(());
		TreeLocationVisitor::visit_with_location_mut(
			&mut node,
			self.root_location,
			|loc, node| {
				match node {
					WebNode::Block(RsxBlock { effect, .. }) => {
						if let Err(err) = std::mem::take(effect).register(loc) {
							result = Err(err);
						}
					}
					WebNode::Element(e) => {
						for a in &mut e.attributes {
							let res = match a {
								RsxAttribute::Block { effect, .. } => {
									std::mem::take(effect).register(loc)
								}
								RsxAttribute::BlockValue { effect, .. } => {
									std::mem::take(effect).register(loc)
								}
								_ => Ok(()),
							};
							if let Err(err) = res {
								result = Err(err);
							}
						}
					}
					_ => {}
				};
			},
		);
		result
	}
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use crate::sigfault::signal;
	use sweet::prelude::*;

	#[test]
	fn registration() {
		let (get, _) = signal(7);
		expect(
			rsx! { <div>value is {get}</div> }
				.xpipe(MountToRsDom)
				.unwrap()
				.xpipe(RegisterEffects::default()),
		)
		.to_be_ok();
	}

	/// This would cause a cannot recursively acquire mutex in wasm
	/// because of wasm panic catch limitations
	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	#[should_panic]
	#[ignore = "too hard to reset DOM_TARGET for every test after poison"]
	fn bad_location() {
		let (get, _) = signal(7);
		let _ = rsx! { <div>value is {get}</div> }
			.xpipe(MountToRsDom)
			.unwrap()
			.xpipe(RegisterEffects::new(TreeLocation::new(10, 10, 10)));
	}


	#[test]
	fn root() {
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
