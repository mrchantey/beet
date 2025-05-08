use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::WorkspacePathBuf;
use sweet::prelude::*;


/// When parsing a file, it may contain multiple rsx templates and style templates:
/// ## Rsx Templates
/// For rust files, each rsx template is defined by an `rsx!` macro, whereas for mdx files
/// the entire file is a single rsx template.
/// ## Style Templates
/// When visiting an rsx template, any style tag without the `scope:verbatim` directive
/// is considered a style template and will be extracted, replaced by a
/// [`TemplateDirective::StylePlaceholder`] directive.
#[derive(Debug, Default)]
pub struct FileTemplates {
	/// A [`TokenStream`] representing a [`ron`] representation of a [`RsxTemplateNode`].
	pub rsx_templates: Vec<(NodeSpan, RsxTemplateNode)>,
	// /// A [`TokenStream`] representing styles extracted from the file.
	pub style_templates: Vec<StyleTemplate>,
}



pub struct FileToTemplates;



impl Pipeline<WorkspacePathBuf, Result<FileTemplates>> for FileToTemplates {
	fn apply(self, path: WorkspacePathBuf) -> Result<FileTemplates> {
		let mut templates = FileTemplates::default();

		match path.extension() {
			Some(ex) if ex == "rs" => path.xpipe(RsToWebTokens),
			Some(ex) if ex == "md" || ex == "mdx" => {
				path.xpipe(MdToWebTokens).map(|v| vec![v])
			}
			_ => Ok(Default::default()),
		}?
		.xmap_each(|(location, web_tokens)| {
			let web_tokens = web_tokens.xpipe(ParseWebTokens::default())?;
			templates
				.style_templates
				.extend(web_tokens.xref().xpipe(WebTokensToStyleTemplates)?);


			let rsx_ron =
				web_tokens.xpipe(WebTokensToRon::default()).to_string();
			let template_node =
				ron::de::from_str::<RsxTemplateNode>(rsx_ron.trim())
					.map_err(|e| ron_cx_err(e, &rsx_ron))?;
			templates.rsx_templates.push((location, template_node));
			Ok(())
		})
		.into_iter()
		.collect::<Result<()>>()?;
		Ok(templates)
	}
}


impl FileToTemplates {


	// fn extract_

}

/// A ron deserialization error with the context of the file and line
fn ron_cx_err(e: ron::error::SpannedError, str: &str) -> anyhow::Error {
	/// how many leading and trailing characters to show in the context of the error
	const CX_SIZE: usize = 8;
	
	let start = e.position.col.saturating_sub(CX_SIZE);
	let end = e.position.col.saturating_add(CX_SIZE);
	let cx = if e.position.line == 1 {
		str[start..end].to_string()
	} else {
		let lines = str.lines().collect::<Vec<_>>();
		let str = lines.get(e.position.line - 1).unwrap_or(&"");
		str[start..end].to_string()
	};
	return anyhow::anyhow!(
		"Failed to parse RsxTemplate from string\nError: {}\nContext: {}\nFull: {}",
		e.code,
		cx,
		str
	);
}
