use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::pin::Pin;


#[derive(Default)]
pub struct RouteFuncsToRsx;

impl
	Pipeline<
		Vec<RouteFunc<DefaultRouteFunc>>,
		Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxNode)>>>>>,
	> for RouteFuncsToRsx
{
	fn apply(
		self,
		routes: Vec<RouteFunc<DefaultRouteFunc>>,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxNode)>>>>> {
		Box::pin(async move {
			futures::future::try_join_all(routes.into_iter().map(
				async |func| {
					let node = (func.func)().await?;
					let info = func.route_info;
					Ok((info, node))
				},
			))
			.await
		})
	}
}

/// allows directly adding a [`RouteFunc`] to the `AppRouter`
impl IntoCollection<Self> for Vec<RouteFunc<DefaultRouteFunc>> {
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let html_dir = args.html_dir.clone();
				Box::pin(async move {
					let routes = self
						.xpipe(RouteFuncsToRsx::default())
						.await?
						.xpipe(ApplyRouteTemplates::default())?;

					// export client islands after templates are applied,
					// at this stage the only required transform is the slots pipeline
					routes
						.clone()
						.into_iter()
						.map(|(info, root)| {
							Ok((info, root.xpipe(ApplySlots::default())?))
						})
						.collect::<Result<Vec<_>>>()?
						.xref()
						.xpipe(RoutesToClientIslandMap::default())?;

					routes
						.xpipe(RoutesToHtml::default())?
						.xpipe(HtmlRoutesToDisk::new(html_dir))?;
					Ok(())
				})
			}));
			// wasm mounting is handled by ClientIslandMountFuncs
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
		let html = crate::test_site::pages::collect()
			.xpipe(RouteFuncsToRsx::default())
			.await
			.unwrap()
			.xpipe(RoutesToHtml::default())
			.unwrap();

		expect(html.len()).to_be(3);
		expect(
			html.iter()
				.find(|(info, _)| info.path.to_string_lossy() == "/docs"),
		)
		.to_be_some();

		expect(
			html.iter().find(|(info, _)| {
				info.path.to_string_lossy() == "/contributing"
			}),
		)
		.to_be_some();


		expect(
			html.iter()
				.find(|(info, _)| info.path.to_string_lossy() == "/")
				.map(|(_, html)| html.clone().xpipe(RenderHtml::default()))
				.unwrap(),
		)
		.to_contain("<!DOCTYPE html>");
	}
}
