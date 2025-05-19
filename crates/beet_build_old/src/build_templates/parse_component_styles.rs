use super::error::Error;
use super::error::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;

/// For each [`LangTemplate`] where the tag is `style`
/// and it is [`StyleScope::Local`], apply its id to all selectors.
pub struct ParseComponentStyles {
	/// The attribute used for selectors, this must match the
	/// one used in [`ApplyStyleIds`]
	attr: String,
}

impl Default for ParseComponentStyles {
	fn default() -> Self {
		ParseComponentStyles {
			attr: ApplyStyleIds::DEFAULT_STYLEID_ATTR.to_string(),
		}
	}
}

impl Pipeline<LangTemplateMap, Result<LangTemplateMap>>
	for ParseComponentStyles
{
	/// Applies scoped style to:
	/// 1. root node
	/// 2. all component nodes
	/// 3. all component slot children
	fn apply(
		self,
		mut template_map: LangTemplateMap,
	) -> Result<LangTemplateMap> {
		template_map
			.values_mut()
			.filter(|template| template.tag == "style")
			.par_bridge()
			.into_par_iter()
			.map(|template| self.apply_styles(template))
			.collect::<Result<Vec<_>>>()?;
		Ok(template_map)
	}
}

impl ParseComponentStyles {
	fn class_name(&self, id: u64) -> String { format!("{}-{}", self.attr, id) }

	fn apply_styles(&self, template: &mut LangTemplate) -> Result<()> {
		// hack to allow for the css unit "em" to be used in the style tag
		// we should put it somewhere else
		template.content = template.content.replace(".em", "em");
		// Parse the stylesheet
		let mut stylesheet =
			StyleSheet::parse(&template.content, ParserOptions::default())
				.map_err(|e| Error::Parse {
					span: template.spans().first().cloned().unwrap_or_default(),
					err: ParseError::Serde(e.to_string()),
				})?;
		let scope = template.directives.style_scope().unwrap_or_default();
		if scope == StyleScope::Local {
			let class_name = self.class_name(template.id);
			stylesheet.rules.0.iter_mut().for_each(|rule| {
				match rule {
					// currently only style rules are supported
					lightningcss::rules::CssRule::Style(style_rule) => {
						style_rule.selectors.0.iter_mut().for_each(
							|selector| {
								selector.append(
								lightningcss::selector::Component::AttributeInNoNamespaceExists {
									local_name: class_name.clone().into(),
									local_name_lower: class_name.clone().into(),
								}
							);
							},
						);
					}
					_ => {}
				}
			});
		}

		#[cfg(debug_assertions)]
		let options = PrinterOptions::default();
		#[cfg(not(debug_assertions))]
		let options = PrinterOptions {
			minify: true,
			..Default::default()
		};

		let new_css = stylesheet
			.to_css(options)
			.map_err(|e| Error::Parse {
				span: template.spans().first().cloned().unwrap_or_default(),
				err: ParseError::Serde(e.to_string()),
			})?
			.code;
		drop(stylesheet);
		template.content = new_css;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use anyhow::Result;
	use sweet::prelude::*;


	fn parse(tokens: WebTokens) -> Result<String> {
		tokens
			.xpipe(ExtractLangTemplates::default())?
			.1
			.xpipe(CollectLangTemplates)?
			.xpipe(ParseComponentStyles::default())?
			.xmap(|map| map.values().next().unwrap().content.clone())
			.xok()
	}

	#[test]
	fn local() {
		web_tokens! {
			<style>body{color:red;}</style>
		}
		.xmap(parse)
		.unwrap()
		.xpect()
		.to_be("body[data-styleid-0] {\n  color: red;\n}\n");
	}
	#[test]
	fn global() {
		web_tokens! {
			<style scope:global> body{} </style>
		}
		.xmap(parse)
		.unwrap()
		.xpect()
		.to_be("body {\n}\n");
	}
}
