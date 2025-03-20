use crate::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;

pub struct StaticRouteCollection;

impl IntoCollection<StaticRoute> for RouteTree<StaticRoute> {
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let html_dir = args.html_dir.clone();
				Box::pin(async move {
					let routes = self
						.pipe(StaticRoutesToRsx::default())
						.await?
						.pipe(ApplyRouteTemplates::default())?;
					// export client islands after templates are applied
					// but before `DefaultTransformations` are applied.
					// i dont think its nessecary but if it turns out to be
					// we can move some pipes around
					(&routes).pipe(RoutesToClientIslandMap::default())?;

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
