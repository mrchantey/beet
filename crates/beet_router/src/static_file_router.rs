use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use sweet::prelude::FsExt;

/// Simple default state for the static server,
/// you will likely outgrow this quickly but it
/// will help get up and running.
#[derive(Debug, Clone)]
pub struct DefaultAppState {
	pub app_name: String,
}

impl Default for DefaultAppState {
	fn default() -> Self {
		Self {
			app_name: "My Site".to_string(),
		}
	}
}
type StaticRsxFunc<T> =
	Box<dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<RsxNode>>>>>;


/// A simple static server, it allows for a state value
/// that can be passed by reference to all routes,
/// this is where site constants can be stored.
pub struct StaticFileRouter<T> {
	/// a constant state that can be used by routes for rendering
	pub state: T,
	pub page_routes: Vec<StaticPageRoute<T>>,
	/// The directory to save the html files to
	pub dst_dir: PathBuf,
}

impl<T: Default> Default for StaticFileRouter<T> {
	fn default() -> Self {
		Self {
			state: Default::default(),
			page_routes: Default::default(),
			dst_dir: "target/client".into(),
		}
	}
}


impl<T: 'static> StaticFileRouter<T> {
	pub fn add_route<M>(
		&mut self,
		(info, route): (RouteInfo, impl StaticPageRouteFunc<T, M>),
	) {
		self.page_routes
			.push(StaticPageRoute::new(info, route.into_rsx_func()));
	}
	pub async fn routes_to_rsx(&self) -> Result<Vec<(RouteInfo, RsxNode)>> {
		futures::future::try_join_all(
			self.page_routes
				.iter()
				.map(|route| self.route_to_rsx(route)),
		)
		.await
	}
	async fn route_to_rsx(
		&self,
		route: &StaticPageRoute<T>,
	) -> Result<(RouteInfo, RsxNode)> {
		let node = route.into_node(&self.state).await?;
		Ok((route.route_info.clone(), node))
	}


	pub async fn routes_to_html(
		&self,
	) -> Result<Vec<(RouteInfo, HtmlDocument)>> {
		let scoped_style = ScopedStyle::default();
		let html = self
			.routes_to_rsx()
			.await?
			.into_iter()
			.map(|(p, mut node)| {
				scoped_style.apply(&mut node)?;
				let doc = RsxToHtml::default().map_node(&node).into_document();
				Ok((p, doc))
			})
			.collect::<Result<Vec<(RouteInfo, HtmlDocument)>>>()?;
		Ok(html)
	}

	/// map the routes to html and save to disk
	pub async fn routes_to_html_files(&self) -> Result<()> {
		FsExt::remove(&self.dst_dir).ok();
		let html = self.routes_to_html().await?;
		for (info, doc) in html {
			let mut path = info.path.clone();
			if path.file_stem().map(|s| s == "index").unwrap_or(false) {
				path.set_extension("html");
			} else {
				// routers expect index.html for any path without an extension
				path.set_extension("");
				path.push("index.html");
			}
			let full_path = self.dst_dir.join(&path);
			println!(
				"writing:\n{}\n{}",
				self.dst_dir.display(),
				full_path.display(),
				// self.dst_dir.join(full_path).display()
			);
			FsExt::write(&full_path, &doc.render())?;
		}
		Ok(())
	}
}

// impl<T: 'static> FileRouter for StaticFileRouter<T> {

// }

pub struct StaticPageRoute<T> {
	/// the function collected and placed in `file_routes.rs`
	pub func: StaticRsxFunc<T>,
	// fn(&T) -> Pin<Box<dyn Future<Output = Result<RsxNode>>>>,
	pub route_info: RouteInfo,
}
impl<T> StaticPageRoute<T> {
	pub fn new(route_info: RouteInfo, func: StaticRsxFunc<T>) -> Self {
		Self { route_info, func }
	}
}


impl<T: 'static> PageRoute for StaticPageRoute<T> {
	type Context = T;
	async fn into_node(&self, context: &Self::Context) -> Result<RsxNode> {
		(self.func)(context).await
	}
}

pub trait StaticPageRouteFunc<T, M>: 'static {
	fn into_rsx_func(&self) -> StaticRsxFunc<T>;
}



impl<F: 'static + Clone + Fn() -> R, R: Rsx, T> StaticPageRouteFunc<T, ()>
	for F
{
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |_context| {
			let func = func.clone();
			Box::pin(async move { Ok(func().into_rsx()) })
		})
	}
}



pub struct WithArgsMarker;


impl<F, T, R> StaticPageRouteFunc<T, WithArgsMarker> for F
where
	R: Rsx,
	T: 'static + Clone,
	F: 'static + Clone + Fn(T) -> R,
{
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |context| {
			let func = func.clone();
			let context = context.clone();
			Box::pin(async move { Ok(func(context).into_rsx()) })
		})
	}
}
impl<F, T, R> StaticPageRouteFunc<T, &WithArgsMarker> for F
where
	R: Rsx,
	T: 'static + Clone,
	F: 'static + Clone + Fn(&T) -> R,
{
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |context| {
			let func = func.clone();
			let context = context.clone();
			Box::pin(async move { Ok(func(&context).into_rsx()) })
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::html::RenderHtml;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut router = DefaultFileRouter::default();
		crate::test_site::test_site_router::collect_file_routes(&mut router);
		let html = router.routes_to_html().await.unwrap();

		expect(html.len()).to_be(2);

		expect(&html[0].0.path.to_string_lossy())
			.to_end_with("routes/contributing.rs");
		expect(&html[0].1.render()).to_be("<!DOCTYPE html><html><head></head><body><div><h1>Beet</h1>\n\t\t\t\tparty time dude!\n\t\t</div></body></html>");
		expect(&html[1].0.path.to_string_lossy())
			.to_end_with("routes/index.rs");
		expect(&html[1].1.render()).to_be("<!DOCTYPE html><html><head></head><body><div><h1>My Site</h1>\n\t\t\t\tparty time!\n\t\t</div></body></html>");
	}
}
