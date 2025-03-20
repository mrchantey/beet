mod export_html;
#[cfg(all(not(target_arch = "wasm32"), feature = "parser"))]
pub mod static_file_router;
pub use export_html::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "parser"))]
pub use static_file_router::*;

use anyhow::Result;
use beet_rsx::rsx::RsxRoot;
use http::Method;
use std::path::PathBuf;
use std::str::FromStr;


pub trait RoutesToRsx {
	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, RsxRoot)>>;
}


#[derive(Debug, Clone)]
pub struct RouteInfo {
	/// the url path
	pub path: PathBuf,
	/// the http method
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
