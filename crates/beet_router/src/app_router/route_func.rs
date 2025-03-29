use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::pin::Pin;

/// A function that has no parameters and returns a [`RsxRoot`].
pub type DefaultRouteFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;


pub struct RouteFunc<T> {
	/// the route path
	pub route_info: RouteInfo,
	pub func: T,
}

impl<T> RsxPipelineTarget for RouteFunc<T> {}

impl<T> RouteFunc<T> {
	pub fn new<M>(
		method: &str,
		route_path: &str,
		func: impl IntoRouteFunc<T, M>,
	) -> Self {
		Self {
			route_info: RouteInfo::new(route_path, method),
			func: func.into_route_func(),
		}
	}

	// pub fn into_route_info(&self) -> RouteInfo {
	// 	RouteInfo::new(self.route_path.clone(), &self.method)
	// }
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
