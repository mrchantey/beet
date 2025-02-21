use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::Path;

pub struct BeetHtml;

impl BeetHtml {
	/// Renders the app to a html file, useful for a SPA setup
	/// where there is only one entrypoint, but it still contains
	/// static html.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn render_to_file<M>(
		app: impl IntoRsxRoot<M>,
		dst: impl AsRef<Path>,
	) -> Result<()> {
		use sweet::prelude::FsExt;
		// the cli built the template map by looking at this file
		let template_map =
			RsxTemplateMap::load(BuildRsxTemplateMap::DEFAULT_TEMPLATES_DST)?;

		// we'll create the app even though its static parts are stale
		// because we need the rusty parts to fill in the html template
		let stale_app = app.into_root();


		// apply the template to the app
		let fresh_app = template_map.apply_template(stale_app)?;

		// build the doc and save it, the web server will detect a change
		// and reload the page.
		let mut doc = RsxToResumableHtml::default().map_root(&fresh_app);
		doc.insert_wasm_script();
		let html = doc.render();
		FsExt::write(dst, &html)?;
		Ok(())
	}
}
