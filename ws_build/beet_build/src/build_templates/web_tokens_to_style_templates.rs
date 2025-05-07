use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;


pub struct WebTokensToStyleTemplates;

impl<T: AsRef<WebTokens>> Pipeline<T, Result<Vec<StyleTemplate>>>
	for WebTokensToStyleTemplates
{
	fn apply(self, web_tokens: T) -> Result<Vec<StyleTemplate>> {
		let web_tokens = web_tokens.as_ref();
		let mut styles = vec![];
		web_tokens.walk_web_tokens::<anyhow::Error>(|node| {
			if let Some(style) = Self::get_style_template(node)? {
				styles.push(style);
			}
			Ok(())
		})?;

		Ok(styles)
	}
}

impl WebTokensToStyleTemplates {
	fn get_style_template(node: &WebTokens) -> Result<Option<StyleTemplate>> {
		match node {
			WebTokens::Element {
				component,
				children,
				self_closing,
			} if component.tag.to_string() == "style" => {
				let scope = StyleScope::from_meta(&component.meta);
				todo!();

				// let scope = component.directives.scope;

				Ok(None)
			}
			_ => Ok(None),
		}
	}
}

#[derive(Debug, Default, Clone, Hash)]
pub struct StyleTemplate {
	/// the scope of the style
	pub scope: StyleScope,
	/// the style template
	pub style: String,
}




#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleScope {
	/// Styles are scoped only to the component
	#[default]
	Local,
	/// Styles are scoped to the global scope
	Global,
}

impl StyleScope {
	pub fn from_meta(meta: &NodeMeta) -> Self {
		if meta.is_global_scope() {
			Self::Global
		} else {
			Self::Local
		}
	}
}
