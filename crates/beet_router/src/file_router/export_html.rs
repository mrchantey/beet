use super::RouteInfo;
use anyhow::Result;
use beet_router_parser::prelude::*;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use sweet::prelude::FsExt;


/// For a given router export each html file, using the templates map if available
pub struct RoutesToHtml {
	/// Location of the `rsx-templates.ron` file
	pub templates_map_path: Option<PathBuf>,
}

impl Default for RoutesToHtml {
	fn default() -> Self {
		Self {
			templates_map_path: Some(
				BuildTemplateMap::DEFAULT_TEMPLATES_MAP_PATH.into(),
			),
		}
	}
}
impl RoutesToHtml {
	/// Create a new instance of `RoutesToHtml` with a custom `templates_map_path`
	pub fn new(templates_map_path: impl Into<PathBuf>) -> Self {
		Self {
			templates_map_path: Some(templates_map_path.into()),
		}
	}
	pub fn without_templates() -> Self {
		Self {
			templates_map_path: None,
		}
	}
}


impl
	RsxPipeline<
		Vec<(RouteInfo, RsxRoot)>,
		Result<Vec<(RouteInfo, HtmlDocument)>>,
	> for RoutesToHtml
{
	fn apply(
		self,
		routes: Vec<(RouteInfo, RsxRoot)>,
	) -> Result<Vec<(RouteInfo, HtmlDocument)>> {
		// if we can't load templates just warn and run without template reload
		let mut template_map = self
			.templates_map_path
			.as_ref()
			.map(|path| {
				RsxTemplateMap::load(&path)
					.map_err(|err| {
						// notify user that we are using routes
						eprintln!(
							"Live reload disabled - Error loading template map at: {:?}\n{:#?}",
							self.templates_map_path, err,
						);
						err
					})
					.ok()
			})
			.flatten();

		let html = routes
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
}

pub struct HtmlRoutesToDisk {
	/// The directory to save the html files to
	pub html_dir: PathBuf,
}
impl HtmlRoutesToDisk {
	/// Create a new instance of `HtmlRoutesToDisk` with a custom `html_dir`
	pub fn new(html_dir: impl Into<PathBuf>) -> Self {
		Self {
			html_dir: html_dir.into(),
		}
	}
}

impl Default for HtmlRoutesToDisk {
	fn default() -> Self {
		Self {
			html_dir: "target/client".into(),
		}
	}
}


impl RsxPipeline<Vec<(RouteInfo, HtmlDocument)>, Result<()>>
	for HtmlRoutesToDisk
{
	fn apply(self, routes: Vec<(RouteInfo, HtmlDocument)>) -> Result<()> {
		let dst = &self.html_dir;
		// in debug mode removing a watched dir breaks FsWatcher
		#[cfg(not(debug_assertions))]
		FsExt::remove(&dst).ok();
		std::fs::create_dir_all(&dst)?;

		let dst = dst.canonicalize()?;
		for (info, doc) in routes.into_iter() {
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
			let pretty = doc.pipe(RenderHtml::pretty())?;
			FsExt::write(&full_path, &pretty)?;
		}


		Ok(())
	}
}
