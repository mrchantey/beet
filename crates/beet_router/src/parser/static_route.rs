use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;


/// A simple static router
#[derive(Default)]
pub struct FileFuncsToRsx;

impl
	RsxPipeline<
		Vec<FileFunc<DefaultFileFunc>>,
		Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxRoot)>>>>>,
	> for FileFuncsToRsx
{
	fn apply(
		self,
		routes: Vec<FileFunc<DefaultFileFunc>>,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, RsxRoot)>>>>> {
		Box::pin(async move {
			futures::future::try_join_all(routes.into_iter().map(
				async |func| {
					let node = (func.func)().await?;
					let info = func.into_route_info();
					Ok((info, node))
				},
			))
			.await
		})
	}
}

/// allows directly adding a `FileFunc` to the `AppRouter`
impl IntoCollection<Self> for Vec<FileFunc<DefaultFileFunc>> {
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let html_dir = args.html_dir.clone();
				Box::pin(async move {
					// TODO we'll codegen to a seperate file
					let routes_mod_path = PathBuf::default();
					let routes = self
						.pipe(FileFuncsToRsx::default())
						.await?
						.pipe(ApplyRouteTemplates::default())?
						.into_iter()
						// TODO this is a hack, we are also applying slots pipeline
						// in RoutesToHtml
						.map(|(info, root)| {
							Ok((info, root.pipe(SlotsPipeline::default())?))
						})
						.collect::<Result<Vec<_>>>()?;

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
		let html = crate::test_site::routes::collect()
			.pipe(FileFuncsToRsx::default())
			.await
			.unwrap()
			.pipe(RoutesToHtml::default())
			.unwrap();

		expect(html.len()).to_be(3);

		expect(&html[0].0.path.to_string_lossy()).to_be("/docs");
		expect(&html[1].0.path.to_string_lossy()).to_be("/contributing");
		expect(&html[2].0.path.to_string_lossy()).to_be("/");
		expect(&html[0].1.clone().pipe(RenderHtml::default()).unwrap())
			.to_contain("<!DOCTYPE html>");
	}
}
