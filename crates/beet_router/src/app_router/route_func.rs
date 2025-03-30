use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::pin::Pin;
use std::sync::Arc;

/// A function that has no parameters and returns a [`RsxRoot`].
pub type DefaultRouteFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;


pub struct RouteFunc<T> {
	/// the route path
	pub route_info: RouteInfo,
	pub func: T,
}

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

pub struct SyncRouteFuncMarker;


impl<F> IntoRouteFunc<DefaultRouteFunc, SyncRouteFuncMarker> for F
where
	F: 'static + Fn() -> RsxRoot,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}


pub struct AsyncRouteFuncMarker;

impl<F> IntoRouteFunc<DefaultRouteFunc, AsyncRouteFuncMarker> for F
where
	F: 'static + AsyncFn() -> RsxRoot,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}

pub struct ResultRouteFuncMarker;

impl<F, E>
	IntoRouteFunc<
		DefaultRouteFunc,
		(SyncRouteFuncMarker, ResultRouteFuncMarker),
	> for F
where
	E: std::error::Error + 'static + Send + Sync,
	F: 'static + Fn() -> Result<RsxRoot, E>,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()?) })
		})
	}
}
impl<F, E>
	IntoRouteFunc<
		DefaultRouteFunc,
		(AsyncRouteFuncMarker, ResultRouteFuncMarker),
	> for F
where
	E: std::error::Error + 'static + Send + Sync,
	F: 'static + AsyncFn() -> Result<RsxRoot, E>,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await?) })
		})
	}
}

pub struct AnyhowRouteFuncMarker;


impl<F>
	IntoRouteFunc<
		DefaultRouteFunc,
		(SyncRouteFuncMarker, AnyhowRouteFuncMarker),
	> for F
where
	F: 'static + Fn() -> anyhow::Result<RsxRoot>,
{
	fn into_route_func(self) -> DefaultRouteFunc {
		let func = Arc::new(self);
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { func() })
		})
	}
}
impl<F>
	IntoRouteFunc<
		DefaultRouteFunc,
		(AsyncRouteFuncMarker, AnyhowRouteFuncMarker),
	> for F
where
	F: 'static + AsyncFn() -> anyhow::Result<RsxRoot>,
{
	fn into_route_func(self) -> DefaultRouteFunc {
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
	use sweet::prelude::*;

	#[test]
	fn works() {
		let _sync: DefaultRouteFunc = || -> RsxRoot {
			rsx! {}
		}
		.into_route_func();
		let _sync_result: DefaultRouteFunc =
			|| -> Result<RsxRoot> { Ok(rsx! {}) }.into_route_func();
		let _async_func: DefaultRouteFunc = async || -> RsxRoot {
			rsx! {}
		}
		.into_route_func();
	}
}
