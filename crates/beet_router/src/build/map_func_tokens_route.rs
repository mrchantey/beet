use crate::prelude::*;
use beet_rsx::prelude::*;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Default, Clone)]
pub struct MapFuncTokensRoute {
	/// A base path to prepend to the route path
	base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	replace: Vec<(String, String)>,
}


impl MapFuncTokensRoute {
	/// Create a new [`MapFuncTokensRoute`] with the given base route.
	pub fn new<S: ToString>(
		base_route: impl Into<PathBuf>,
		replace: impl IntoIterator<Item = (S, S)>,
	) -> Self {
		Self {
			base_route: Some(RoutePath::new(base_route)),
			replace: replace
				.into_iter()
				.map(|(a, b)| (a.to_string(), b.to_string()))
				.collect(),
		}
	}
	pub fn base_route(mut self, base_route: impl Into<PathBuf>) -> Self {
		self.base_route = Some(RoutePath::new(base_route));
		self
	}
	pub fn replace<S: ToString>(
		mut self,
		replace: impl IntoIterator<Item = (S, S)>,
	) -> Self {
		self.replace = replace
			.into_iter()
			.map(|(a, b)| (a.to_string(), b.to_string()))
			.collect();
		self
	}
}


impl RsxPipeline<Vec<FuncTokens>> for MapFuncTokensRoute {
	fn apply(self, funcs: Vec<FuncTokens>) -> Vec<FuncTokens> {
		funcs
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
				for (needle, replacement) in &self.replace {
					route_path = route_path.replace(needle, replacement);
				}
				func.route_info.path = RoutePath::new(route_path);
				func
			})
			.collect()
	}
}

#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use beet_rsx::prelude::*;
	// use quote::ToTokens;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		// let _route_funcs = FileGroup::test_site_routes()
		// 	.bpipe(FileGroupToFuncTokens::default())
		// 	.unwrap()
		// 	.bpipe(FuncTokensToRoutes::default());
	}
}
