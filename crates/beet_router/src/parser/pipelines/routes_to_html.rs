use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use sweet::prelude::*;

/// For a given router export each html file, using the templates map if available
#[derive(Default)]
pub struct RoutesToHtml;


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
		let html = routes
			.into_iter()
			.map(|(route, root)| {
				// only hydrate if we have templates
				// we already warned otherwise
				let doc = root.pipe(RsxToHtmlDocument::default())?;
				Ok((route.clone(), doc))
			})
			.collect::<Result<Vec<_>>>()?;
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
			// pretty rendering currently breaks text node logic
			let str = doc.pipe(RenderHtml::default())?;
			FsExt::write(&full_path, &str)?;
		}


		Ok(())
	}
}
