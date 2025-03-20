use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::pin::Pin;

pub type StaticRoute =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;

/// A simple static router
#[derive(Default)]
pub struct StaticRoutesToRsx;

impl RsxPipelineTarget for RouteTree<StaticRoute> {}

impl
	RsxPipeline<
		RouteTree<StaticRoute>,
		Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxRoot)>>>>>,
	> for StaticRoutesToRsx
{
	fn apply(
		self,
		routes: RouteTree<StaticRoute>,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxRoot)>>>>> {
		Box::pin(async move {
			futures::future::try_join_all(routes.flatten().into_iter().map(
				async |(info, func)| {
					let node = (func)().await?;
					Ok((info, node))
				},
			))
			.await
		})
	}
}

impl<F: 'static + Clone + Fn() -> RsxRoot> IntoRoute<StaticRoute, ()> for F {
	fn into_route(&self) -> StaticRoute {
		let func = self.clone();
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}

pub struct AsyncStaticRouteMarker;

impl<F> IntoRoute<StaticRoute, AsyncStaticRouteMarker> for F
where
	F: 'static + Clone + AsyncFn() -> RsxRoot,
{
	fn into_route(&self) -> StaticRoute {
		let func = self.clone();
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let html = crate::test_site::routes::collect()
			.pipe(StaticRoutesToRsx::default())
			.await
			.unwrap()
			.pipe(RoutesToHtml::default())
			.unwrap();

		expect(html.len()).to_be(3);

		expect(&html[0].0.path.to_string_lossy()).to_be("/contributing");
		expect(&html[0].1.clone().pipe(RenderHtml::default()).unwrap()).to_be("<!DOCTYPE html><html><head></head><body><div><h1 data-beet-rsx-idx=\"4\">Test Site</h1>party time dude!</div></body></html>");
		expect(&html[1].0.path.to_string_lossy()).to_be("/");
		expect(&html[1].1.clone().pipe(RenderHtml::default()).unwrap()).to_be("<!DOCTYPE html><html><head></head><body><div><h1 data-beet-rsx-idx=\"4\">Test Site</h1>party time!</div></body></html>");
	}
}
