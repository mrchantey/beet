use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;
// use std::sync::atomic::AtomicUsize;
// use std::sync::atomic::Ordering;

// static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

// fn next_id() -> usize { ID_COUNTER.fetch_add(1, Ordering::Relaxed) }

/// Visit web tokens and replace style templates with a [`TemplateDirective::StylePlaceholder`]
#[derive(Default)]
pub struct ExtractStyleTemplates;

impl Pipeline<WebTokens, Result<(WebTokens, Vec<StyleTemplate>)>>
	for ExtractStyleTemplates
{
	fn apply(
		self,
		mut web_tokens: WebTokens,
	) -> Result<(WebTokens, Vec<StyleTemplate>)> {
		let mut styles = vec![];
		web_tokens.walk_web_tokens_mut::<anyhow::Error>(|node| {
			if let Some(style) = Self::try_extract_style(node)? {
				styles.push(style);
			}
			Ok(())
		})?;

		Ok((web_tokens, styles))
	}
}

impl ExtractStyleTemplates {
	fn try_extract_style(
		node: &mut WebTokens,
	) -> Result<Option<StyleTemplate>> {
		match node {
			WebTokens::Element {
				component,
				children,
				self_closing: _,
			} if component.tag.as_str() == "style" => {
				let scope = component.meta.style_scope().unwrap_or_default();
				if children.is_empty() || component.meta.is_inline() {
					return Ok(None);
				}
				let WebTokens::Text { value, meta: _ } =
					*std::mem::take(children)
				else {
					anyhow::bail!(
						"Style templates must have one text child,\
						opt out of style templating with the is:inline directive"
					);
				};

				let style = StyleTemplate {
					scope: scope.clone(),
					inner_text: value.to_string(),
				};
				let mut hasher = RapidHasher::default_const();
				style.hash(&mut hasher);
				let hash = hasher.finish();

				component.meta.template_directives.push(
					TemplateDirective::StylePlaceholder { content_hash: hash },
				);

				Ok(Some(style))
			}
			_ => Ok(None),
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct StyleTemplate {
	/// the scope of the style
	pub scope: StyleScope,
	/// the style template
	pub inner_text: String,
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn ignores() {
		web_tokens! {<div client:load/>}
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
		expect(tokens.meta().style_placeholder())
			.to_be(Some(3981390837452447284));
		let WebTokens::Element { children, .. } = tokens else {
			panic!()
		};
		expect(children.is_empty()).to_be_true();
	}
}
