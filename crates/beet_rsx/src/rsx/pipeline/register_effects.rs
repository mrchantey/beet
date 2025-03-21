use crate::prelude::*;

use anyhow::Result;

/// Registers all effects for a node and its children
#[derive(Default)]
pub struct RegisterEffects {
	/// The initial location used by the [`TreeLocationVisitor`]
	pub root_location: TreeLocation,
}

impl<T: RsxPipelineTarget + AsMut<RsxNode>> RsxPipeline<T, Result<()>>
	for RegisterEffects
{
	fn apply(self, mut node: T) -> Result<()> {
		let mut result = Ok(());

		TreeLocationVisitor::visit_mut(node.as_mut(), |loc, node| {
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
		});
		result
	}
}
