use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::PathBuf;

/// Load an [`RsxTemplateMap`] and apply the templates to each route
pub struct ApplyRouteTemplates {
	/// Location of the `rsx-templates.ron` file
	pub templates_map_path: PathBuf,
}

impl Default for ApplyRouteTemplates {
	fn default() -> Self {
		Self {
			templates_map_path: BuildTemplateMap::DEFAULT_TEMPLATES_MAP_PATH
				.into(),
		}
	}
}
impl ApplyRouteTemplates {
	/// Create a new instance of `RoutesToHtml` with a custom `templates_map_path`
	pub fn new(templates_map_path: impl Into<PathBuf>) -> Self {
		Self {
			templates_map_path: templates_map_path.into(),
		}
	}
}


impl Pipeline<Vec<(RouteInfo, RsxNode)>, Result<Vec<(RouteInfo, RsxNode)>>>
	for ApplyRouteTemplates
{
	fn apply(
		self,
		routes: Vec<(RouteInfo, RsxNode)>,
	) -> Result<Vec<(RouteInfo, RsxNode)>> {
		let template_map = RsxTemplateMap::load(&self.templates_map_path)
			.map_err(|err| {
				// notify user that we are using routes
				anyhow::anyhow!(
					"Live reload disabled - Error loading template map at: {:?}\n{:#?}",
					self.templates_map_path,
					err,
				)
			})?;

		routes
			.into_iter()
			.map(|(route, root)| Ok((route, root.xpipe(&template_map)?)))
			.collect()
	}
}
