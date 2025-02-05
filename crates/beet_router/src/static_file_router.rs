use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use sweet::prelude::FsExt;
use sweet::prelude::ReadFile;

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
	Box<dyn Fn(&T) -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;


/// A simple static server, it allows for a state value
/// that can be passed by reference to all routes,
/// this is where site constants can be stored.
pub struct StaticFileRouter<T> {
	/// a constant state that can be used by routes for rendering
	pub state: T,
	pub page_routes: Vec<StaticPageRoute<T>>,
	/// The directory to save the html files to
	pub dst_dir: PathBuf,
	pub templates_src: PathBuf,
}

impl<T: Default> Default for StaticFileRouter<T> {
	fn default() -> Self {
		Self {
			state: Default::default(),
			page_routes: Default::default(),
			dst_dir: "target/client".into(),
			// keep in sync with BuildRsxTemplates
			templates_src: "target/rsx-templates.ron".into(),
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
		route: &StaticPageRoute<T>,
	) -> Result<(RouteInfo, RsxRoot)> {
		let node = route.into_node(&self.state).await?;
		Ok((route.route_info.clone(), node))
	}

	fn load_template_map(
		&self,
	) -> Result<HashMap<RsxLocation, RsxTemplateNode>> {
		let tokens = ReadFile::to_string(&self.templates_src)?;
		let templates: HashMap<RsxLocation, RsxTemplateNode> =
			ron::de::from_str(&tokens.to_string())?;
		Ok(templates)
	}

	/// try applying templates, otherwise warn and use
	/// the compiled rsx
	pub async fn routes_to_html(
		&self,
	) -> Result<Vec<(RouteInfo, HtmlDocument)>> {
		let scoped_style = ScopedStyle::default();

		let mut template_map = self
			.load_template_map()
			.map_err(|err| {
				eprintln!("No templates found at {:?}", self.templates_src);
				err
			})
			.ok();

		let mut apply_templates = |root: RsxRoot| -> Result<RsxRoot> {
			if let Some(templates) = &mut template_map {
				let mut split = root.split_hydration()?;
				split.template =
					templates.remove(&split.location).ok_or_else(|| {
						anyhow::anyhow!(
							"No template found for {:?}",
							&split.location
						)
					})?;
				let location = split.location.clone();
				// println!("split: {:#?}", split);
				Ok(RsxRoot::join_hydration(split).map_err(|err| {
					anyhow::anyhow!(
						"Failed to join hydration at location: {:#?}\n{:?}",
						location,
						err
					)
				})?)
			} else {
				// we already warned about missing templates
				Ok(root)
			}
		};

		let html = self
			.routes_to_rsx()
			.await?
			.into_iter()
			.map(|(route, root)| {
				let mut root = apply_templates(root)?;
				scoped_style.apply(&mut root)?;
				let doc = RsxToHtml::default().map_node(&root).into_document();
				Ok((route, doc))
			})
			.collect::<Result<Vec<(RouteInfo, HtmlDocument)>>>()?;
		Ok(html)
	}

	/// map the routes to html and save to disk
	pub async fn routes_to_html_files(&self) -> Result<()> {
		let dst = &self.dst_dir;
		// in debug mode this breaks FsWatcher
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
	async fn into_node(&self, context: &Self::Context) -> Result<RsxRoot> {
		(self.func)(context).await
	}
}

pub trait StaticPageRouteFunc<T, M>: 'static {
	fn into_rsx_func(&self) -> StaticRsxFunc<T>;
}



impl<F: 'static + Clone + Fn() -> RsxRoot, T> StaticPageRouteFunc<T, ()> for F {
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |_context| {
			let func = func.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}



pub struct WithArgsMarker;


impl<F, T> StaticPageRouteFunc<T, WithArgsMarker> for F
where
	T: 'static + Clone,
	F: 'static + Clone + Fn(T) -> RsxRoot,
{
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |context| {
			let func = func.clone();
			let context = context.clone();
			Box::pin(async move { Ok(func(context)) })
		})
	}
}
impl<F, T> StaticPageRouteFunc<T, &WithArgsMarker> for F
where
	T: 'static + Clone,
	F: 'static + Clone + Fn(&T) -> RsxRoot,
{
	fn into_rsx_func(&self) -> StaticRsxFunc<T> {
		let func = self.clone();
		Box::new(move |context| {
			let func = func.clone();
			let context = context.clone();
			Box::pin(async move { Ok(func(&context)) })
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
		crate::test_site::routes::collect_file_routes(&mut router);
		let html = router.routes_to_html().await.unwrap();

		expect(html.len()).to_be(2);

		expect(&html[0].0.path.to_string_lossy()).to_be("/contributing");
		expect(&html[0].1.render()).to_be("<!DOCTYPE html><html><head></head><body><div><h1>Beet</h1>party time dude!</div></body></html>");
		expect(&html[1].0.path.to_string_lossy()).to_be("/");
		expect(&html[1].1.render()).to_be("<!DOCTYPE html><html><head></head><body><div><h1>My Site</h1>party time!</div></body></html>");
	}
}
