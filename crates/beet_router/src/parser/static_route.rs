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

pub struct StaticRouteCollection;

impl IntoCollection<StaticRoute> for RouteTree<StaticRoute> {
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let html_dir = args.html_dir.clone();
				Box::pin(async move {
					let routes_mod_path = self.mod_path.clone();
					let routes = self
						.pipe(StaticRoutesToRsx::default())
						.await?
						.pipe(ApplyRouteTemplates::default())?;
					// export client islands after templates are applied
					// but before `DefaultTransformations` are applied.
					// i dont think its nessecary because islands only register effect
					// but if it turns out to be we can move some pipes around
					(&routes)
						.pipe(RoutesToClientIslandMap::new(routes_mod_path))?;

					routes
						.pipe(RoutesToHtml::default())?
						.pipe(HtmlRoutesToDisk::new(html_dir))?;
					Ok(())
				})
			}));
			#[cfg(target_arch = "wasm32")]
			{
				todo!("use window.location to determine hydration route");
			}
		}
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
