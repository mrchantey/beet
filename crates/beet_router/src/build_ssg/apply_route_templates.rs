use crate::prelude::*;
use anyhow::Result;
use beet_template::prelude::*;
use std::path::PathBuf;

/// Load an [`NodeTemplateMap`] and apply the templates to each route
pub struct ApplyRouteTemplates {
	/// Location of the [`NodeTemplateMap`] file
	pub node_templates_path: PathBuf,
	/// Location of the [`LangTemplateMap`] file
	pub lang_templates_path: PathBuf,
}

impl Default for ApplyRouteTemplates {
	fn default() -> Self {
		Self {
			node_templates_path: default_paths::NODE_TEMPLATE_MAP.into(),
			lang_templates_path: default_paths::LANG_TEMPLATE_MAP.into(),
		}
	}
}
impl ApplyRouteTemplates {}

impl Pipeline<Vec<(RouteInfo, WebNode)>, Result<Vec<(RouteInfo, WebNode)>>>
	for ApplyRouteTemplates
{
	fn apply(
		self,
		routes: Vec<(RouteInfo, WebNode)>,
	) -> Result<Vec<(RouteInfo, WebNode)>> {
		let node_template_map = NodeTemplateMap::load(
			&self.node_templates_path,
		)
		.map_err(|err| {
			anyhow::anyhow!(
				"Error loading node template map at: {:?}\n{:#?}",
				self.node_templates_path,
				err,
			)
		})?;
		let lang_template_map = LangTemplateMap::load(
			&self.lang_templates_path,
		)
		.map_err(|err| {
			anyhow::anyhow!(
				"Error loading lang template map at: {:?}\n{:#?}",
				self.lang_templates_path,
				err,
			)
		})?;

		routes
			.into_iter()
			.map(|(route, root)| {
				Ok((
					route,
					root.xpipe(&node_template_map)?
						.xpipe(&lang_template_map)?
						.xpipe(ApplyStyleIds::default()),
				))
			})
			.collect()
	}
}
