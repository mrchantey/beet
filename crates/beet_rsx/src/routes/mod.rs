use crate::rsx::RsxNode;
use http::Method;
use std::path::PathBuf;
use std::str::FromStr;


/// A type used by `beet_router` to store route information.
pub struct Route {
	/// the file path
	pub path: PathBuf,
	pub method: Method,
	pub handler: Box<dyn Fn() -> RsxNode>,
}
impl Route {
	/// the method used by `beet_router`
	pub fn build(
		path: &str,
		method: &str,
		handler: impl 'static + Fn() -> RsxNode,
	) -> Self {
		Self {
			path: PathBuf::from(path),
			method: Method::from_str(method).unwrap(),
			handler: Box::new(handler),
		}
	}
}
