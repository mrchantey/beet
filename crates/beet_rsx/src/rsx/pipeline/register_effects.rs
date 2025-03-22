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

impl<T: RsxPipelineTarget + AsMut<RsxNode>> RsxPipeline<T, Result<()>>
	for RegisterEffects
{
	fn apply(self, mut node: T) -> Result<()> {
		let mut result = Ok(());

		TreeLocationVisitor::visit_with_options_mut(
			node.as_mut(),
			self.root_location,
			Default::default(),
			|loc, node| {
				// println!(
				// 	"registering effect at loc: {:?}:{:?}",
				// 	loc,
				// 	node.discriminant()
				// );
				match node {
					RsxNode::Block(RsxBlock { effect, .. }) => {
						if let Err(err) = effect.take().register(loc) {
							result = Err(err);
						}
					}
					RsxNode::Element(e) => {
						for a in &mut e.attributes {
							let res = match a {
								RsxAttribute::Block { effect, .. } => {
									effect.take().register(loc)
								}
								RsxAttribute::BlockValue { effect, .. } => {
									effect.take().register(loc)
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
				.pipe(MountRsDom)
				.unwrap()
				.pipe(RegisterEffects::default()),
		)
		.to_be_ok();
	}

	/// This would cause a cannot recursively acquire mutex in wasm
	/// because of wasm panic catch limitations
	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	#[should_panic]
	fn bad_location() {
		let (get, _) = signal(7);
		let _ = rsx! { <div>value is {get}</div> }
			.pipe(MountRsDom)
			.unwrap()
			.pipe(RegisterEffects::new(TreeLocation::new(10, 10, 10)));
	}


	#[test]
	fn root() {
		let (get, set) = signal(7);

		rsx! { <div>value is {get}</div> }
			.pipe(MountRsDom)
			.unwrap()
			.pipe(RegisterEffects::default())
			.unwrap();
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 7</div>");
		set(8);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 8</div>");
		set(9);
		expect(&DomTarget::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 9</div>");
	}
}
