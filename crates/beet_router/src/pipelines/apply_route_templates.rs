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


impl RsxPipeline<Vec<(RouteInfo, RsxRoot)>, Result<Vec<(RouteInfo, RsxRoot)>>>
	for ApplyRouteTemplates
{
	fn apply(
		self,
		routes: Vec<(RouteInfo, RsxRoot)>,
	) -> Result<Vec<(RouteInfo, RsxRoot)>> {
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
			.map(|(route, root)| {
				// TODO check if inside templates_root_dir.
				// if so, error, otherwise do nothing
				let root = template_map.apply_template(root)?;
				Ok((route, root))
			})
			.collect()
	}
}
