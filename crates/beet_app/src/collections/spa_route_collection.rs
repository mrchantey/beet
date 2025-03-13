use crate::prelude::*;
use beet_router::prelude::*;
use beet_rsx::prelude::*;



pub struct SpaRouteCollection;

impl<F> IntoCollection<SpaRouteCollection> for F
where
	F: 'static + Send + Sync + FnOnce() -> RsxRoot,
{
	fn into_collection(self) -> impl Collection {
		move |app: &mut BeetApp| {
			#[cfg(not(target_arch = "wasm32"))]
			app.on_run_static.push(Box::new(move |args| {
				let path = args.html_dir.join("index.html");
				Box::pin(async move {
					SpaTemplate::render_to_file(self, path).unwrap();

					Ok(())
				})
			}));

			#[cfg(target_arch = "wasm32")]
			app.on_run_wasm.push(Box::new(move |_args| {
				BeetDom::hydrate(self);
				Ok(())
			}));
		}
	}
}
