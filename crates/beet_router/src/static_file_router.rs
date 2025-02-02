use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::pin::Pin;



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
#[derive(Default)]
pub struct StaticFileRouter<T> {
	/// a constant state that can be used by routes for rendering
	pub state: T,
	pub page_routes: Vec<StaticPageRoute<T>>,
}


impl<T: 'static> StaticFileRouter<T> {
	pub fn add_route<M>(
		&mut self,
		(info, route): (RouteInfo, impl StaticPageRouteFunc<T, M>),
	) {
		self.page_routes
			.push(StaticPageRoute::new(info, route.into_rsx_func()));
	}
	pub async fn collect_rsx(&self) -> Result<Vec<(String, RsxNode)>> {
		futures::future::try_join_all(
			self.page_routes
				.iter()
				.map(|route| self.collect_route_rsx(route)),
		)
		.await
	}
	async fn collect_route_rsx(
		&self,
		route: &StaticPageRoute<T>,
	) -> Result<(String, RsxNode)> {
		let node = route.render(&self.state).await?;
		Ok((route.route_info.path.to_string_lossy().to_string(), node))
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
	async fn render(&self, context: &Self::Context) -> Result<RsxNode> {
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
	use beet_rsx::html::RsxToHtml;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut router = DefaultFileRouter::default();
		crate::test_site::test_site_router::collect_file_routes(&mut router);
		let files = router
			.collect_rsx()
			.await
			.unwrap()
			.into_iter()
			.map(|(p, f)| (p, RsxToHtml::render_body(&f)))
			.collect::<Vec<_>>();

		expect(files.len()).to_be(2);

		expect(&files[0].1).to_be("<html><div><h1>Beet</h1>\n\t\t\t\tparty time dude!\n\t\t</div></html>");
		expect(&files[1].1).to_be("<html><div><h1>My Site</h1>\n\t\t\t\tparty time!\n\t\t</div></html>");
	}
}
