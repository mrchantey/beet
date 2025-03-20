use crate::prelude::*;

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
					ExportHtml {
						html_dir,
						..Default::default()
					}
					.routes_to_html(&mut router)
					.await?;
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
