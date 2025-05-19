use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::pin::Pin;
use std::sync::Arc;

/// A function that has no parameters and returns a [`WebNode`].
pub type RsxRouteFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<WebNode>>>>>;


pub struct RouteFunc<T> {
	/// the route path
	pub route_info: RouteInfo,
	pub func: T,
}

impl<T> std::fmt::Debug for RouteFunc<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouteFunc")
			.field("route_info", &self.route_info)
			.field("func", &std::any::type_name::<T>())
			.finish()
	}
}

impl<T> RouteFunc<T> {
	pub fn new<M>(
		route_info: RouteInfo,
		func: impl IntoRouteFunc<T, M>,
	) -> Self {
		Self {
			route_info,
			func: func.into_route_func(),
		}
	}

	/// Map this route func into another route func,
	/// maintaining the same route info.
	pub fn map_func<T2: IntoRouteFunc<T, M>, M>(
		self,
		func: impl FnOnce(T) -> T2,
	) -> Self {
		RouteFunc {
			route_info: self.route_info,
			func: func(self.func).into_route_func(),
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

// self
impl<T: 'static> IntoRouteFunc<T, Self> for T {
	fn into_route_func(self) -> T { self }
}

pub struct SyncRouteFuncMarker;


impl<F> IntoRouteFunc<RsxRouteFunc, SyncRouteFuncMarker> for F
where
	F: 'static + Fn() -> WebNode,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}


pub struct AsyncRouteFuncMarker;

impl<F> IntoRouteFunc<RsxRouteFunc, AsyncRouteFuncMarker> for F
where
	F: 'static + AsyncFn() -> WebNode,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}

pub struct ResultRouteFuncMarker;

impl<F, E>
	IntoRouteFunc<RsxRouteFunc, (SyncRouteFuncMarker, ResultRouteFuncMarker)> for F
where
	E: std::error::Error + 'static + Send + Sync,
	F: 'static + Fn() -> Result<WebNode, E>,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()?) })
		})
	}
}
impl<F, E>
	IntoRouteFunc<RsxRouteFunc, (AsyncRouteFuncMarker, ResultRouteFuncMarker)>
	for F
where
	E: std::error::Error + 'static + Send + Sync,
	F: 'static + AsyncFn() -> Result<WebNode, E>,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await?) })
		})
	}
}

pub struct AnyhowRouteFuncMarker;


impl<F>
	IntoRouteFunc<RsxRouteFunc, (SyncRouteFuncMarker, AnyhowRouteFuncMarker)> for F
where
	F: 'static + Fn() -> anyhow::Result<WebNode>,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { func() })
		})
	}
}
impl<F>
	IntoRouteFunc<RsxRouteFunc, (AsyncRouteFuncMarker, AnyhowRouteFuncMarker)>
	for F
where
	F: 'static + AsyncFn() -> anyhow::Result<WebNode>,
{
	fn into_route_func(self) -> RsxRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { func().await })
		})
	}
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use anyhow::Result;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let _sync: RsxRouteFunc = || -> WebNode {
			rsx! {}
		}
		.into_route_func();
		let _sync_result: RsxRouteFunc =
			|| -> Result<WebNode> { Ok(rsx! {}) }.into_route_func();
		let _async_func: RsxRouteFunc = async || -> WebNode {
			rsx! {}
		}
		.into_route_func();
	}
}
