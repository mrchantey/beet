use crate::prelude::*;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapFuncTokens {
	/// A base path to prepend to the route path
	base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	#[serde(default)]
	replace_route: Vec<ReplaceRoute>,
}
/// Replace some part of the route path with another string
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaceRoute {
	/// The string to replace
	from: String,
	/// The string to replace with
	to: String,
}

impl Default for MapFuncTokens {
	fn default() -> Self {
		Self {
			base_route: None,
			replace_route: vec![],
		}
	}
}


impl MapFuncTokens {
	pub fn base_route(mut self, base_route: impl Into<PathBuf>) -> Self {
		self.base_route = Some(RoutePath::new(base_route));
		self
	}
	pub fn replace_route<S: ToString>(
		mut self,
		replace: impl IntoIterator<Item = (S, S)>,
	) -> Self {
		self.replace_route = replace
			.into_iter()
			.map(|(a, b)| ReplaceRoute {
				from: a.to_string(),
				to: b.to_string(),
			})
			.collect();
		self
	}
}


impl Pipeline<FuncTokensGroup> for MapFuncTokens {
	fn apply(self, group: FuncTokensGroup) -> FuncTokensGroup {
		group
			.funcs
			.into_iter()
			.map(|mut func| {
				let mut route_path = if let Some(base_route) = &self.base_route
				{
					base_route
						.join(&func.route_info.path)
						.to_string_lossy()
						.to_string()
				} else {
					func.route_info.path.to_string_lossy().to_string()
				};
				for ReplaceRoute { from, to } in &self.replace_route {
					route_path = route_path.replace(from, to);
				}
				func.route_info.path = RoutePath::new(route_path);
				func
			})
			.collect::<Vec<_>>()
			.into()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	// use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			&FileGroup::test_site()
				.with_filter(GlobFilter::default().with_include("*.mockup.*"))
				.xpipe(FileGroupToFuncTokens::default())
				.unwrap()
				.xpipe(
					MapFuncTokens::default()
						.base_route("/design")
						.replace_route([(".mockup", "")]),
				)
				.xmap_each(|func| func.route_info.path.to_string()),
		)
		.to_contain_element(
			&"/design/components/mock_widgets/mock_button".into(),
		)
		.to_contain_element(&"/design/components/test_layout".into())
		.to_contain_element(&"/design".into());
	}
}
