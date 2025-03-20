use super::RouteInfo;
use super::RoutesToRsx;
use anyhow::Result;
use beet_router_parser::prelude::*;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use sweet::prelude::FsExt;


/// For a given router export each html file
pub struct ExportHtml {
	/// The directory to save the html files to
	pub html_dir: PathBuf,
	/// Location of the `rsx-templates.ron` file
	pub templates_map_path: PathBuf,
}

impl Default for ExportHtml {
	fn default() -> Self {
		Self {
			html_dir: "target/client".into(),
			templates_map_path: BuildTemplateMap::DEFAULT_TEMPLATES_MAP_PATH
				.into(),
		}
	}
}


impl ExportHtml {
	/// try applying templates, otherwise warn and use
	/// the compiled rsx
	pub async fn routes_to_html(
		&self,
		router: &mut impl RoutesToRsx,
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

		let html = router
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
	pub async fn routes_to_html_files(
		&self,
		router: &mut impl RoutesToRsx,
	) -> Result<()> {
		let dst = &self.html_dir;
		// in debug mode removing a watched dir breaks FsWatcher
		#[cfg(not(debug_assertions))]
		FsExt::remove(&dst).ok();
		std::fs::create_dir_all(&dst)?;

		let dst = dst.canonicalize()?;
		for (info, doc) in self.routes_to_html(router).await? {
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
