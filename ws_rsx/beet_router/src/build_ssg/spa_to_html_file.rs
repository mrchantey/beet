use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::PathBuf;
use sweet::prelude::*;


/// Renders the app to a html file, useful for a SPA setup
/// where there is only one entrypoint, but it still contains
/// static html.
pub struct SpaToHtmlFile {
	dst: PathBuf,
}

impl SpaToHtmlFile {
	/// Create a new instance of `SpaTemplate` with a custom `dst`
	pub fn new(dst: impl Into<PathBuf>) -> Self { Self { dst: dst.into() } }
}

impl Pipeline<WebNode, Result<()>> for SpaToHtmlFile {
	fn apply(self, app: WebNode) -> Result<()> {
		// we'll create the app even though its static parts are stale
		// because we need the rusty parts to fill in the html template
		// apply the template to the app
		let html = app
			// the cli built the template map by looking at this file
			.xpipe(&NodeTemplateMap::load(default_paths::NODE_TEMPLATE_MAP)?)?
			.xpipe(&LangTemplateMap::load(default_paths::LANG_TEMPLATE_MAP)?)?
			.xpipe(RsxToHtmlDocument::default())?
			.xpipe(RenderHtmlEscaped::default());

		// build the doc and save it, the web server will detect a change
		// and reload the page.
		FsExt::write(&self.dst, &html)?;
		Ok(())
	}
}
