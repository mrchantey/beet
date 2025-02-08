use crate::prelude::*;
use anyhow::Result;


/// Serializable version of a [`RsxRoot`]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateRoot {
	pub node: RsxTemplateNode,
	pub location: RsxLocation,
}



impl RsxTemplateRoot {
	#[cfg(feature = "serde")]
	pub fn from_ron(ron: &str) -> Result<Self> {
		ron::de::from_str(ron).map_err(Into::into)
	}


	pub fn from_rsx(node: &RsxRoot) -> Result<Self> {
		let location = node.location.clone();
		let node = RsxTemplateNode::from_rsx_node(&node.node)?;
		Ok(Self { node, location })
	}

	/// Create an RsxRoot from a template and hydrated nodes.
	/// 		todo!("this is wrong, we need template map for each component?;
	pub fn into_rsx(self, hydrated: &mut RsxHydratedMap) -> Result<RsxRoot> {
		let node = self.node.into_rsx_node(hydrated).map_err(|err| {
			anyhow::anyhow!(
				"Failed to hydrate template at location: {:#?}\n{}",
				self.location,
				err
			)
		})?;
		Ok(RsxRoot {
			node,
			location: self.location,
		})
	}
}
