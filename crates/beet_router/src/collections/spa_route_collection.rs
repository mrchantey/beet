use crate::prelude::*;
use beet_rsx::prelude::*;


pub struct SpaRouteCollection;


impl SpaRouteCollection {
	pub fn mount() -> Self {
		todo!(
			"this should not use BeetDom and instead follow the same client islands workflow as file based routes"
		);
	}
}


impl<F> IntoCollection<SpaRouteCollection> for F
where
	F: 'static + Send + Sync + FnOnce() -> RsxNode,
{
	fn into_collection(self) -> impl Collection {
		#[allow(unused)]
		move |app: &mut AppRouter| {
			#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
			app.on_run_static.push(Box::new(move |args| {
				let path = args.html_dir.join("index.html");
				Box::pin(async move {
					self().xpipe(SpaToHtmlFile::new(path)).unwrap();
					Ok(())
				})
			}));
		}
	}
}
