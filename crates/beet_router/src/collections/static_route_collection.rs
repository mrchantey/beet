use crate::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;

pub struct StaticRouteCollection;

impl<F> IntoCollection<StaticRouteCollection> for F
where
	F: 'static + FnOnce(&mut StaticFileRouter),
{
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let mut router = StaticFileRouter::default();
				self(&mut router);
				let html_dir = args.html_dir.clone();
				Box::pin(async move {
					let routes = router
						.routes_to_rsx()
						.await?
						.pipe(ApplyRouteTemplates::new(html_dir))?;
					// export client islands after templates are applied
					// but before `DefaultTransformations` are applied.
					// i dont think its nessecary but if it turns out to be
					// we can move some pipes around
					(&routes).pipe(RoutesToClientIslandMap::default())?;

					routes
						.pipe(RoutesToHtml::default())?
						.pipe(HtmlRoutesToDisk::default())?;
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
