use anyhow::Result;
use beet_rsx::prelude::*;
use http::Method;
use std::path::PathBuf;
use std::str::FromStr;


/// This trait serves as both a collection of helper functions
/// and a marker that the implementer should be able to handle
/// [ParseFileRoutes]. It must have the following
///
/// - a function called add_route(info: RouteInfo,F) where F is
/// the get function etc.
///
pub trait FileRouter {
	/// Page routes are routes that can return [RsxNode].
	type PageRoute: PageRoute;
	/// collect all page routes
	fn page_routes(&self) -> impl Iterator<Item = &Self::PageRoute>;

	async fn render(&self) -> Result<Vec<RsxNode>> {
		futures::future::try_join_all(
			self.page_routes()
				.into_iter()
				.map(|route| self.render_route(route)),
		)
		.await
	}

	async fn render_route(&self, route: &Self::PageRoute) -> Result<RsxNode>;
}

pub trait PageRoute {
	type Context;


	async fn render(&self, context: &Self::Context) -> Result<RsxNode>;
}


pub struct RouteInfo {
	pub path: PathBuf,
	pub method: Method,
}
impl RouteInfo {
	/// the method used by `beet_router`
	pub fn new(path: &str, method: &str) -> Self {
		Self {
			path: PathBuf::from(path),
			method: Method::from_str(method).unwrap(),
		}
	}
}


