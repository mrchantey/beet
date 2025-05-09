use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;

/// Visit web tokens and replace style templates with a [`TemplateDirective::StylePlaceholder`]
#[derive(Default)]
pub struct ExtractLangTemplates;

impl Pipeline<WebTokens, Result<(WebTokens, Vec<(FileSpan, LangTemplate)>)>>
	for ExtractLangTemplates
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

impl ExtractLangTemplates {
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

				let Some(content) =
					lang_content(&component.meta, std::mem::take(children))?
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



fn lang_content(
	meta: &NodeMeta,
	children: WebTokens,
) -> Result<Option<LangContent>> {
	match (children.is_empty(), meta.src_directive(), children) {
		(true, None, _) => {
			// empty
			Ok(None)
		}
		(true, Some(src), _) => {
			// src directive
			Ok(Some(LangContent::File(meta.span().file().join(src))))
		}
		(false, None, WebTokens::Text { value, .. }) => {
			// inline text
			Ok(Some(LangContent::Inline(value.to_string())))
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


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn ignores() {
		web_tokens! {<div>foobar</div>}
			.xpipe(ExtractLangTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style is:inline>div{}</style>}
			.xpipe(ExtractLangTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style/>}
			.xpipe(ExtractLangTemplates)
			.unwrap()
			.1
			.xpect()
			.to_be(vec![]);
		web_tokens! {<style></style>}
			.xpipe(ExtractLangTemplates)
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
		.xpipe(ExtractLangTemplates)
		.xpect()
		.to_be_ok();
		// source and inner text
		web_tokens! {<style src="./foo">foo{}</style>
		}
		.xpipe(ExtractLangTemplates)
		.xpect()
		.to_be_err();
	}

	#[test]
	#[ignore = "multiple children wont error because \
	rstml treats style tag inner as a single text node"]
	fn multi_children() {
		web_tokens! {<style><div/><br/></style>
		}
		.xpipe(ExtractLangTemplates)
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
		.xpipe(ExtractLangTemplates)
		.unwrap();
		expect(styles.len()).to_be(1);
		expect(tokens.meta().lang_template()).to_be_some();
		let WebTokens::Element { children, .. } = tokens else {
			panic!()
		};
		expect(children.is_empty()).to_be_true();
	}
}
