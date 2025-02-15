use crate::prelude::*;
use anyhow::Result;



/// A cli router will visit all files in a crate and collect each
/// rsx! macro into a ron structure of this type using
/// [beet_rsx_parser::RstmlToRsxTemplate]. Because it operates on the level
/// of *tokens* and *ron format*, it is actually unaware of this type directly.
/// The advantage of this approach is we can build templates statically without
/// a compile step.
///
///
/// When joining an [RsxTemplateRoot] with an [RustyPartMap],
/// we need the entire [RsxTemplateMap] to resolve components.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateMap(pub HashMap<RsxLocation, RsxTemplateRoot>);


impl RsxTemplateMap {
	/// Load the template map serialized by [beet_rsx_parser::RstmlToRsxTemplate]
	#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
	pub fn load(src: &std::path::Path) -> Result<Self> {
		use sweet::prelude::ReadFile;
		{
			let tokens = ReadFile::to_string(src)?;
			let this: Self = ron::de::from_str(&tokens.to_string())?;
			Result::Ok(this)
		}
	}

	/// used for testing, load directly from a collection of template roots.
	pub fn from_template_roots(roots: Vec<RsxTemplateRoot>) -> Self {
		Self(
			roots
				.into_iter()
				.map(|root| (root.location.clone(), root))
				.collect(),
		)
	}

	// should live elsewhere, maybe RustyPart
	pub fn hydrate(&mut self, root: RsxRoot) -> Result<RsxRoot> {
		let mut hydrated = RustyPartMap::collect(root.node)?;
		let location = root.location;
		// i think here we need to pass the whole map for component template reloading
		let template = self.remove(&location).ok_or_else(|| {
			anyhow::anyhow!("No template found for {:?}", &location)
		})?;
		let node = template.hydrate(&mut hydrated)?;
		Ok(node)
	}
}
