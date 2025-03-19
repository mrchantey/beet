use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use sweet::prelude::FsExt;

type StaticRsxFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;

/// A simple static router, it allows for a state value
/// that can be passed by reference to all routes,
/// this is where site constants can be stored.
pub struct StaticFileRouter {
	pub page_routes: Vec<StaticPageRoute>,
	/// The directory to save the html files to
	pub html_dir: PathBuf,
	/// Location of the `rsx-templates.ron` file
	pub templates_map_path: PathBuf,
}

impl Default for StaticFileRouter {
	fn default() -> Self {
		Self {
			page_routes: Default::default(),
			html_dir: "target/client".into(),
			templates_map_path: BuildTemplateMap::DEFAULT_TEMPLATES_MAP_PATH
				.into(),
		}
	}
}


impl StaticFileRouter {
	pub fn add_route<M>(
		&mut self,
		(info, route): (RouteInfo, impl IntoStaticRsxFunc<M>),
	) {
		self.page_routes
			.push(StaticPageRoute::new(info, route.into_rsx_func()));
	}
	pub async fn routes_to_rsx(&self) -> Result<Vec<(RouteInfo, RsxRoot)>> {
		futures::future::try_join_all(
			self.page_routes
				.iter()
				.map(|route| self.route_to_rsx(route)),
		)
		.await
	}
	async fn route_to_rsx(
		&self,
		route: &StaticPageRoute,
	) -> Result<(RouteInfo, RsxRoot)> {
		let node = (route.func)().await?;
		Ok((route.route_info.clone(), node))
	}


	/// try applying templates, otherwise warn and use
	/// the compiled rsx
	pub async fn routes_to_html(
		&self,
	) -> Result<Vec<(RouteInfo, HtmlDocument)>> {
		// if we can't load templates just warn and run without template reload
		let mut template_map = RsxTemplateMap::load(&self.templates_map_path)
			.map_err(|err| {
				// notify user that we are using routes
				eprintln!(
					"Live reload disabled - Error loading template map at: {:?}\n{:#?}",
					self.templates_map_path, err,
				);
				err
			})
			.ok();

		let html = self
			.routes_to_rsx()
			.await?
			.into_iter()
			.map(|(route, mut root)| {
				// only hydrate if we have templates
				// we already warned otherwise
				if let Some(map) = &mut template_map {
					// TODO check if inside templates_root_dir.
					// if so, error, otherwise do nothing
					root = map.apply_template(root)?;
				}
				let doc = root.pipe(RsxToHtmlDocument::default())?;
				Ok((route, doc))
			})
			.collect::<Result<Vec<(RouteInfo, HtmlDocument)>>>()?;
		Ok(html)
	}

	/// Calls [Self::routes_to_html] and writes the html to disk
	pub async fn routes_to_html_files(&self) -> Result<()> {
		let dst = &self.html_dir;
		// in debug mode removing a watched dir breaks FsWatcher
		#[cfg(not(debug_assertions))]
		FsExt::remove(&dst).ok();
		std::fs::create_dir_all(&dst)?;

		let dst = dst.canonicalize()?;
		for (info, doc) in self.routes_to_html().await? {
			let mut path = info.path.clone();
			// map foo/index.rs to foo/index.html
			if path.file_stem().map(|s| s == "index").unwrap_or(false) {
				path.set_extension("html");
			} else {
				// map foo/contributing.rs to foo/contributing/index.html
				path.set_extension("");
				path.push("index.html");
			}
			path = path.strip_prefix("/").unwrap().to_path_buf();
			let full_path = &dst.join(path);
			FsExt::write(&full_path, &doc.pipe(RenderHtml::pretty())?)?;
		}
		Ok(())
	}
}


pub struct StaticPageRoute {
	/// the function collected and placed in `file_routes.rs`
	pub func: StaticRsxFunc,
	// fn(&T) -> Pin<Box<dyn Future<Output = Result<RsxNode>>>>,
	pub route_info: RouteInfo,
}
impl StaticPageRoute {
	pub fn new(route_info: RouteInfo, func: StaticRsxFunc) -> Self {
		Self { route_info, func }
	}
}


pub trait IntoStaticRsxFunc<M>: 'static {
	fn into_rsx_func(&self) -> StaticRsxFunc;
}



impl<F: 'static + Clone + Fn() -> RsxRoot> IntoStaticRsxFunc<()> for F {
	fn into_rsx_func(&self) -> StaticRsxFunc {
		let func = self.clone();
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}



pub struct AsyncStaticRouteMarker;


impl<F> IntoStaticRsxFunc<AsyncStaticRouteMarker> for F
where
	F: 'static + Clone + AsyncFn() -> RsxRoot,
{
	fn into_rsx_func(&self) -> StaticRsxFunc {
		let func = self.clone();
		Box::new(move || {
			let func = func.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut router = StaticFileRouter::default();
		crate::test_site::routes::collect_file_routes(&mut router);
		let html = router.routes_to_html().await.unwrap();

		expect(html.len()).to_be(3);

		expect(&html[0].0.path.to_string_lossy()).to_be("/contributing");
		expect(&html[0].1.clone().pipe(RenderHtml::default()).unwrap()).to_be("<!DOCTYPE html><html><head></head><body><div><h1 data-beet-rsx-idx=\"4\">Test Site</h1>party time dude!</div></body></html>");
		expect(&html[1].0.path.to_string_lossy()).to_be("/");
		expect(&html[1].1.clone().pipe(RenderHtml::default()).unwrap()).to_be("<!DOCTYPE html><html><head></head><body><div><h1 data-beet-rsx-idx=\"4\">Test Site</h1>party time!</div></body></html>");
	}
}
