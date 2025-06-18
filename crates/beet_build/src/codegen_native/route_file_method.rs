#[allow(unused_imports)]
use crate::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;

/// Tokens for a function that may be used as a route.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct RouteFileMethod {
	///
	/// Whether this handler has an associated `meta_` method,
	/// ie for `my_route::post()` this would be `my_route::meta_post()`.
	pub meta: RouteFileMethodMeta,
	/// A reasonable route path generated from this file's local path,
	/// and a method matching either the functions signature, or
	/// `get` in the case of single file routes like markdown.
	pub route_info: RouteInfo,
}
impl AsRef<RouteFileMethod> for RouteFileMethod {
	fn as_ref(&self) -> &RouteFileMethod { self }
}


impl RouteFileMethod {
	pub fn new(route_info: impl Into<RouteInfo>) -> Self {
		Self {
			route_info: route_info.into(),
			meta: Default::default(),
		}
	}
	pub fn new_with_config(
		route_info: impl Into<RouteInfo>,
		meta: RouteFileMethodMeta,
	) -> Self {
		Self {
			route_info: route_info.into(),
			meta,
		}
	}
	pub fn from_path(
		local_path: impl AsRef<std::path::Path>,
		method: HttpMethod,
	) -> Self {
		let route = RoutePath::from_file_path(local_path).unwrap();
		Self::new(RouteInfo::new(route, method))
	}
}

/// Specify the location of the meta for this route file method.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RouteFileMethodMeta {
	/// A config method exists for this route file method,
	/// ie `my_route::config_post()`.
	Method,
	/// A config method exists for this route file, ie `my_route::config()`.
	File,
	/// No config method exists for this route file, fall back to
	/// the group level or default.
	#[default]
	FileGroup,
}
