use crate::prelude::*;
use beet_router::prelude::*;

pub struct FileRouteCollection;

impl<F> IntoCollection<FileRouteCollection> for F
where
	F: 'static + FnOnce(&mut DefaultFileRouter),
{
	fn into_collection(self) -> impl Collection {
		move |app: &mut AppRouter| {
			#[cfg(not(target_arch = "wasm32"))]
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
			#[cfg(target_arch = "wasm32")]
			{
				todo!("use window.location to determine hydration route");
			}
		}
	}
}
