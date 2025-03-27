use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use std::pin::Pin;

/// A function that has no parameters and returns a [`RsxRoot`].
pub type DefaultRouteFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;


pub struct RouteFunc<T> {
	/// The path relative to its root, the [`FileGroup::src`] it was collected from.
	/// This is useful for generating route paths.
	pub local_path: PathBuf,
	/// the route path
	pub route_path: RoutePath,
	/// The function name, ie `get`
	pub name: String,
	pub func: T,
}

impl<T> RsxPipelineTarget for RouteFunc<T> {}

impl<T> RouteFunc<T> {
	pub fn new<M>(
		name: &str,
		local_path: &str,
		route_path: &str,
		func: impl IntoRouteFunc<T, M>,
	) -> Self {
		Self {
			name: name.into(),
			local_path: local_path.into(),
			route_path: RoutePath::new(route_path),
			func: func.into_route_func(),
		}
	}

	pub fn into_route_info(&self) -> RouteInfo {
		RouteInfo::new(self.route_path.clone(), &self.name)
	}
}

/// A mechanic that allows great flexibility in the kinds of
/// functions that can be collected.
pub trait IntoRouteFunc<T, M>: 'static {
	fn into_route_func(self) -> T;
}



impl<F> IntoRouteFunc<DefaultRouteFunc, ()> for F
where
	F: 'static + Clone + Fn() -> RsxRoot,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		Box::new(move || {
			// why clone?
			let func = self.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}


pub struct AsyncRouteFuncMarker;

impl<F> IntoRouteFunc<DefaultRouteFunc, AsyncRouteFuncMarker> for F
where
	F: 'static + Clone + AsyncFn() -> RsxRoot,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		Box::new(move || {
			let func = self.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}
