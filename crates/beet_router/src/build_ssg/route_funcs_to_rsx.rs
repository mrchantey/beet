use crate::prelude::*;
use anyhow::Result;
use beet_template::prelude::*;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;


pub struct RouteFuncsToHtml {
	pub html_dir: PathBuf,
}

impl RouteFuncsToHtml {
	/// Create a new instance of `RouteFuncsToHtml` with a custom `html_dir`
	pub fn new(html_dir: impl Into<PathBuf>) -> Self {
		Self {
			html_dir: html_dir.into(),
		}
	}
}

impl
	Pipeline<
		Vec<RouteFunc<RsxRouteFunc>>,
		Pin<Box<dyn Future<Output = Result<()>>>>,
	> for RouteFuncsToHtml
{
	fn apply(
		self,
		routes: Vec<RouteFunc<RsxRouteFunc>>,
	) -> Pin<Box<dyn Future<Output = Result<()>>>> {
		let html_dir = self.html_dir;
		Box::pin(async move {
			// TODO load template maps here, not for each route
			let routes = routes
				.xpipe(RouteFuncsToRsx::default())
				.await?
				.xpipe(ApplyRouteTemplates::default())?;

			// routes.iter().for_each(|(_, root)| {
			// 	VisitWebNode::walk(&root, |node| {
			// 		if node.lang_template().is_some() {
			// 			println!("its a lang template: {:?}", node.meta());
			// 		} else {
			// 			// println!("its a lang template: {:?}");
			// 		}
			// 	})
			// });


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
	}
}


#[derive(Default)]
struct RouteFuncsToRsx;

impl
	Pipeline<
		Vec<RouteFunc<RsxRouteFunc>>,
		Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, WebNode)>>>>>,
	> for RouteFuncsToRsx
{
	fn apply(
		self,
		routes: Vec<RouteFunc<RsxRouteFunc>>,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(RouteInfo, WebNode)>>>>> {
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

#[cfg(test)]
mod test {
	use super::RouteFuncsToRsx;
	use crate::prelude::*;
	use beet_template::prelude::*;
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
				.map(|(_, html)| html.clone().xpipe(ToHtml::default()))
				.unwrap(),
		)
		.to_contain("<!DOCTYPE html>");
	}
}
