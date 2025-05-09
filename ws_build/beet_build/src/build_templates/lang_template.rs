use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use rapidhash::RapidHasher;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::WorkspacePathBuf;
// use std::sync::atomic::AtomicUsize;
// use std::sync::atomic::Ordering;

// static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

// fn next_id() -> usize { ID_COUNTER.fetch_add(1, Ordering::Relaxed) }

/// Visit web tokens and replace style templates with a [`TemplateDirective::StylePlaceholder`]
#[derive(Default)]
pub struct ExtractStyleTemplates;

impl Pipeline<WebTokens, Result<(WebTokens, Vec<(FileSpan, LangTemplate)>)>>
	for ExtractStyleTemplates
{
	fn apply(
		self,
		mut web_tokens: WebTokens,
	) -> Result<(WebTokens, Vec<(FileSpan, LangTemplate)>)> {
		let mut templates = vec![];
		web_tokens.walk_web_tokens_mut::<anyhow::Error>(|node| {
			if let Some(style) = Self::try_extract_style(node)? {
				templates.push(style);
			}
			Ok(())
		})?;

		Ok((web_tokens, templates))
	}
}

impl ExtractStyleTemplates {
	fn try_extract_style(
		node: &mut WebTokens,
	) -> Result<Option<(FileSpan, LangTemplate)>> {
		match node {
			WebTokens::Element {
				component,
				children,
				self_closing: _,
			} => {
				// filter out by:
				// - only style and script tags
				// - no is:inline directive
				if !["style", "script"]
					.iter()
					.any(|t| *t == component.tag.as_str())
					|| component.meta.is_inline()
				{
					return Ok(None);
				}

				let Some(content) = LangContent::from_element(
					&component.meta,
					std::mem::take(children),
				)?
				else {
					// empty element
					return Ok(None);
				};
				let lang_template = LangTemplate::new(
					component.meta.directives().to_vec(),
					content,
				);

				component.push_directive(TemplateDirective::LangTemplate {
					content_hash: lang_template.hash_self(),
				});

				Ok(Some((component.meta().span().clone(), lang_template)))
			}
			_ => Ok(None),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LangTemplate {
	/// the scope of the style
	pub directives: Vec<TemplateDirective>,
	/// the child text of the element, may be empty
	/// for src templates
	pub content: LangContent,
}

impl LangTemplate {
	/// Hash the content of the template
	pub fn hash_self(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		self.hash(&mut hasher);
		hasher.finish()
	}
}

/// The content of a style template, either inline or a file path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LangContent {
	/// Inner text of an elment: `<script>alert("hello")</script>`
	Inline(String),
	/// A path to a file: `<script src="./foo.js" />`
	File(WorkspacePathBuf),
}

impl LangContent {
	fn from_element(
		meta: &NodeMeta,
		children: WebTokens,
	) -> Result<Option<Self>> {
		match (children.is_empty(), meta.src_directive(), children) {
			(true, None, _) => {
				// empty
				Ok(None)
			}
			(true, Some(src), _) => {
				// src directive
				Ok(Some(Self::File(meta.span().file().join(src))))
			}
			(false, None, WebTokens::Text { value, .. }) => {
				// inline text
				Ok(Some(Self::Inline(value.to_string())))
			}
			(false, Some(_), _) => {
				// inline directive
				anyhow::bail!("elements with src directive must be empty");
			}
			(false, None, _) => {
				// inline directive
				anyhow::bail!(
					"script and style elements must only have one text child"
				);
			}
		}
	}
}


impl LangTemplate {
	pub fn new(
		directives: Vec<TemplateDirective>,
		content: LangContent,
	) -> Self {
		Self {
			directives,
			content,
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
			.xpipe(ExtractStyleTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style is:inline>div{}</style>}
			.xpipe(ExtractStyleTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style/>}
			.xpipe(ExtractStyleTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style></style>}
			.xpipe(ExtractStyleTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
	}


	#[test]
	#[ignore = "multiple children wont error because \
	rstml treats style tag inner as a single text node"]
	fn multi_children() {
		web_tokens! {<style><div/><br/></style>
		}
		.xpipe(ExtractStyleTemplates)
		.xpect()
		.to_be_err();
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
		.xpipe(ExtractStyleTemplates)
		.unwrap();
		expect(styles.len()).to_be(1);
		expect(tokens.meta().lang_template()).to_be_some();
		let WebTokens::Element { children, .. } = tokens else {
			panic!()
		};
		expect(children.is_empty()).to_be_true();
	}
}
