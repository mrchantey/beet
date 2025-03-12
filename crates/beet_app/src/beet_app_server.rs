use crate::prelude::*;
use beet_router::DefaultFileRouter;
use beet_server::axum::Router;


pub struct AxumRouterPluginMarker;

impl IntoCollection<AxumRouterPluginMarker> for Router {
	fn into_collection(self) -> impl Collection {
		move |app: &mut BeetApp| {
			app.router = std::mem::take(&mut app.router).merge(self);
		}
	}
}


/// currently used by collect_routes
pub struct FileRouteCollectionPluginMarker;

impl<F> IntoCollection<FileRouteCollectionPluginMarker> for F
where
	F: 'static + FnOnce(&mut DefaultFileRouter),
{
	fn into_collection(self) -> impl Collection {
		move |app: &mut BeetApp| {
			app.on_run_static.push(Box::new(move |args| {
				let mut router = DefaultFileRouter {
					html_dir: args.html_dir.clone(),
					..Default::default()
				};
				self(&mut router);
				Box::pin(async move {
					router.routes_to_html_files().await?;
					Ok(())
				})
			}));
		}
	}
}
