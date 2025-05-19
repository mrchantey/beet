use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use rapidhash::RapidHasher;
use sweet::prelude::WorkspacePathBuf;

/// Visit web tokens and replace style templates with a [`TemplateDirective::StylePlaceholder`]
pub struct ExtractLangTemplates {
	pub tags: Vec<String>,
}
impl Default for ExtractLangTemplates {
	fn default() -> Self {
		Self {
			tags: RemoveLangTemplates::default_tags(),
		}
	}
}

impl Pipeline<WebTokens, Result<(WebTokens, Vec<ExtractedLangTemplate>)>>
	for ExtractLangTemplates
{
	fn apply(
		self,
		mut web_tokens: WebTokens,
	) -> Result<(WebTokens, Vec<ExtractedLangTemplate>)> {
		let mut templates = vec![];
		web_tokens.walk_web_tokens_mut::<anyhow::Error>(|node| {
			if let Some(template) = self.try_extract_template(node)? {
				templates.push(template);
			}
			Ok(())
		})?;

		Ok((web_tokens, templates))
	}
}
/// An intermediate representation of an extracted template,
/// if its a file the file is not loaded yet, so we only do that
/// once per occurance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ExtractedLangTemplate {
	pub span: FileSpan,
	pub content: ExtractedLangContent,
	pub tag: String,
	pub directives: Vec<TemplateDirectiveEnum>,
	pub content_hash: LangContentHash,
}

impl ExtractedLangTemplate {
	/// Create the [`ExtractedLangTemplate::content_hash`] hash
	pub fn content_hash(
		tag: &str,
		content: &ExtractedLangContent,
		directives: &[TemplateDirectiveEnum],
	) -> LangContentHash {
		let mut hasher = RapidHasher::default_const();
		tag.hash(&mut hasher);
		content.hash(&mut hasher);
		directives.hash(&mut hasher);
		LangContentHash::new(hasher.finish())
	}
}
/// The content of a style template, either inline or a file path. The
/// content of file paths is resolved lazily by the
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum ExtractedLangContent {
	/// Inner text of an elment: `<script>alert("hello")</script>`
	Inline(String),
	/// A path to a file: `<script src="./foo.js" />`
	File(WorkspacePathBuf),
}

impl ExtractedLangContent {}

impl ExtractLangTemplates {
	fn try_extract_template(
		&self,
		node: &mut WebTokens,
	) -> Result<Option<ExtractedLangTemplate>> {
		match node {
			WebTokens::Element {
				component,
				children,
				self_closing: _,
			} => {
				// filter out by:
				// - only style and script tags
				// - no is:inline directive
				// logic must be consistent with RemoveLangTemplates
				if !self.tags.iter().any(|t| *t == component.tag.as_str())
					|| component.meta.is_inline()
				{
					return Ok(None);
				}

				let Some(content) =
					lang_content(&component.meta, std::mem::take(children))?
				else {
					// empty element, remove it
					*node = Default::default();
					return Ok(None);
				};
				let tag = component.tag.as_str().to_string();
				let directives = component.meta.directives().to_vec();
				let content_hash = ExtractedLangTemplate::content_hash(
					&tag,
					&content,
					&directives,
				);
				let span = component.meta.span().clone();

				let lang_template = ExtractedLangTemplate {
					tag,
					directives,
					content,
					content_hash,
					span,
				};


				component.push_directive(TemplateDirectiveEnum::LangTemplate {
					content_hash,
				});

				Ok(Some(lang_template))
			}
			_ => Ok(None),
		}
	}
}



fn lang_content(
	meta: &NodeMeta,
	children: WebTokens,
) -> Result<Option<ExtractedLangContent>> {
	match (children.is_empty(), meta.src_directive(), children) {
		(true, None, _) => {
			// empty
			Ok(None)
		}
		(true, Some(src), _) => {
			// src directive
			Ok(Some(ExtractedLangContent::File(WorkspacePathBuf::new(
				meta.span()
					.file()
					.parent()
					.map(|parent| parent.join(src))
					.unwrap_or(PathBuf::from(src)),
			))))
		}
		(false, None, WebTokens::Text { value, .. }) => {
			// inline text
			Ok(Some(ExtractedLangContent::Inline(value.to_string())))
		}
		(false, Some(_), _) => {
			// src directive with children
			anyhow::bail!("elements with src directive must be empty");
		}
		(false, None, _) => {
			// multiple children
			anyhow::bail!(
				"script and style elements must only have one text child"
			);
		}
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn ignores() {
		web_tokens! {<div>foobar</div>}
			.xpipe(ExtractLangTemplates::default())
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style is:inline>div{}</style>}
			.xpipe(ExtractLangTemplates::default())
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style/>}
			.xpipe(ExtractLangTemplates::default())
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style></style>}
			.xpipe(ExtractLangTemplates::default())
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
	}


	#[test]
	fn errors() {
		// empty is ignored
		web_tokens! {<style></style>
		}
		.xpipe(ExtractLangTemplates::default())
		.xpect()
		.to_be_ok();
		// source and inner text
		web_tokens! {<style src="./foo">foo{}</style>
		}
		.xpipe(ExtractLangTemplates::default())
		.xpect()
		.to_be_err();
		// multiple children wont error because
		// rstml treats style tag inner as a single text node
		// web_tokens! {<style><div/><br/></style>
		// }
		// .xpipe(ExtractLangTemplates::default())
		// .xpect()
		// .to_be_err();
	}


	#[test]
	fn works() {
		let (tokens, styles) = web_tokens! {
			<style scope:global>
				div{
					color: red;
				}
			</style>
		}
		.xpipe(ExtractLangTemplates::default())
		.unwrap();
		expect(styles.len()).to_be(1);
		expect(tokens.meta().lang_template()).to_be_some();
		let WebTokens::Element { children, .. } = tokens else {
			panic!()
		};
		expect(children.is_empty()).to_be_true();
	}
}
