use crate::prelude::*;
use anyhow::Result;



///	Mapping of each component or route to a template.
///
/// When joining an [RsxTemplateRoot] with an [RsxHydratedMap],
/// we need the entire [RsxTemplateMap] to resolve components.
///
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateMap(pub HashMap<RsxLocation, RsxTemplateRoot>);


impl RsxTemplateMap {
	/// used by routers, load a serialized template map
	#[cfg(not(target_arch = "wasm32"))]
	pub fn load(src: &std::path::Path) -> Result<Self> {
		use sweet::prelude::ReadFile;
		{
			let tokens = ReadFile::to_string(src)?;
			let this: Self = ron::de::from_str(&tokens.to_string())?;
			Result::Ok(this)
		}
	}

	/// used for testing
	pub fn from_template_roots(roots: Vec<RsxTemplateRoot>) -> Self {
		Self(
			roots
				.into_iter()
				.map(|root| (root.location.clone(), root))
				.collect(),
		)
	}

	pub fn hydrate(&mut self, root: RsxRoot) -> Result<RsxRoot> {
		let mut hydrated = RsxHydratedMap::collect(root.node)?;
		let location = root.location;
		let template = self.remove(&location).ok_or_else(|| {
			anyhow::anyhow!("No template found for {:?}", &location)
		})?;
		let node = template.into_rsx(&mut hydrated)?;
		Ok(node)
	}
}
