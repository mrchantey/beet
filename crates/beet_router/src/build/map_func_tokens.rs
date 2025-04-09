use crate::prelude::*;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use syn::Expr;

/// Helper for common route mapping
#[derive(Clone)]
pub struct MapFuncTokens<F> {
	/// A base path to prepend to the route path
	base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	replace_route: Vec<(String, String)>,
	/// A function called for each [`FuncTokens::func`],
	/// mapping its expression to a new one.
	wrap_func: Option<F>,
}


impl Default for MapFuncTokens<fn(syn::Expr) -> syn::Expr> {
	fn default() -> Self {
		Self {
			base_route: None,
			replace_route: vec![],
			wrap_func: None,
		}
	}
}


impl<F> MapFuncTokens<F> {
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
			.map(|(a, b)| (a.to_string(), b.to_string()))
			.collect();
		self
	}

	pub fn wrap_func<F2: Fn(Expr) -> Expr>(
		self,
		func: F2,
	) -> MapFuncTokens<F2> {
		MapFuncTokens {
			base_route: self.base_route,
			replace_route: self.replace_route,
			wrap_func: Some(func),
		}
	}
}


impl<F: Fn(Expr) -> Expr> Pipeline<Vec<FuncTokens>> for MapFuncTokens<F> {
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
				for (needle, replacement) in &self.replace_route {
					route_path = route_path.replace(needle, replacement);
				}
				func.route_info.path = RoutePath::new(route_path);
				if let Some(wrap_func) = &self.wrap_func {
					func.func = wrap_func(func.func);
				}
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
		// 	.xpipe(FileGroupToFuncTokens::default())
		// 	.unwrap()
		// 	.xpipe(FuncTokensToRoutes::default());
	}
}
