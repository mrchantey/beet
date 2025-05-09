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
/// When visiting an rsx template, any style tag without the `is:inline` directive
/// is considered a style template and will be extracted, replaced by a
/// [`TemplateDirective::StylePlaceholder`] directive.
#[derive(Debug, Default)]
pub struct FileTemplates {
	/// All collected rsx node templates
	pub node_templates: Vec<WebNodeTemplate>,
	/// All collected style templates
	pub lang_templates: Vec<(FileSpan, LangTemplate)>,
}


#[derive(Debug, Default)]
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
		.xmap_each(|web_tokens| {
			let web_tokens = web_tokens.xpipe(ParseWebTokens::default())?;

			// this will replace the style templates with a placeholder
			let (web_tokens, styles) =
				web_tokens.xpipe(ExtractLangTemplates::default())?;
			let template_node =
				web_tokens.xpipe(WebTokensToTemplate::default());


			templates.node_templates.push(template_node);
			templates.lang_templates.extend(styles);
			Ok(())
		})
		.into_iter()
		.collect::<Result<()>>()?;
		Ok(templates)
	}
}

/// A ron deserialization error with the context of the file and line
#[allow(unused)]
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
