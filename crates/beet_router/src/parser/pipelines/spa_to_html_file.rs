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

impl RsxPipeline<RsxNode, Result<()>> for SpaToHtmlFile {
	fn apply(self, app: RsxNode) -> Result<()> {
		// the cli built the template map by looking at this file
		let template_map =
			RsxTemplateMap::load(BuildTemplateMap::DEFAULT_TEMPLATES_MAP_PATH)?;

		// we'll create the app even though its static parts are stale
		// because we need the rusty parts to fill in the html template
		// apply the template to the app
		let html = app
			.bpipe(&template_map)?
			.bpipe(RsxToHtmlDocument::default())?
			.bpipe(RenderHtml::default())?;

		// build the doc and save it, the web server will detect a change
		// and reload the page.
		FsExt::write(&self.dst, &html)?;
		Ok(())
	}
}
