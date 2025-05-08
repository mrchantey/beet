use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use std::convert::Infallible;
use sweet::prelude::Pipeline;


/// For each [`ElementTokens`], read its [`attributes`](ElementTokens::attributes) and extract them
/// into the [`directives`](ElementTokens::directives) field.
#[derive(Default)]
pub struct ExtractTemplateDirectives;

impl<T: ElementTokensVisitor<Infallible>> Pipeline<T, Result<T>>
	for ExtractTemplateDirectives
{
	fn apply(self, mut node: T) -> Result<T> {
		node.walk_rsx_tokens(parse_node)?;
		Ok(node)
	}
}

/// remove template directives from attributes,
/// and add them to the directives field
fn parse_node(
	ElementTokens {
		attributes, meta, ..
	}: &mut ElementTokens,
) -> Result<(), Infallible> {
	attributes.retain(|attr| {
		if let Some(directive) = attr_to_template_directive(attr) {
			meta.template_directives.push(directive);
			return false;
		}
		true
	});
	Ok(())
}

fn attr_to_template_directive(
	attr: &RsxAttributeTokens,
) -> Option<TemplateDirective> {
	match attr {
		RsxAttributeTokens::Key { key } => match key.as_str() {
			"client:load" => Some(TemplateDirective::ClientLoad),
			"scope:local" => {
				Some(TemplateDirective::StyleScope(StyleScope::Local))
			}
			"scope:global" => {
				Some(TemplateDirective::StyleScope(StyleScope::Global))
			}
			"scope:verbatim" => {
				Some(TemplateDirective::StyleScope(StyleScope::Verbatim))
			}
			"style:cascade" => Some(TemplateDirective::StyleCascade),
			runtime_key if runtime_key.starts_with("runtime:") => {
				let Some(suffix) = runtime_key.split(':').nth(1) else {
					return None;
				};
				return Some(TemplateDirective::Runtime(suffix.to_string()));
			}
			custom_key if custom_key.contains(':') => {
				let mut parts = custom_key.split(':');
				let prefix = parts.next().unwrap_or_default().to_string();
				let suffix = parts.next().unwrap_or_default().to_string();
				Some(TemplateDirective::Custom {
					prefix,
					suffix,
					value: None,
				})
			}
			_attr => None,
		},
		// only key value pairs where the value is a string are valid
		// templates
		RsxAttributeTokens::KeyValue { key, value }
			if let Some(value) = RsxAttributeTokens::try_lit_str(value) =>
		{
			match key.as_str() {
				"slot" => Some(TemplateDirective::Slot(value)),
				"src" if value.starts_with('.') => {
					// alternatively we could use an ignore approach
					// if ["/", "http://", "https://"]
					// .iter()
					// .all(|p| val.starts_with(p) == false)
					Some(TemplateDirective::FsSrc(value))
				}
				custom_key if custom_key.contains(':') => {
					let mut parts = custom_key.split(':');
					let prefix = parts.next().unwrap_or_default().to_string();
					let suffix = parts.next().unwrap_or_default().to_string();
					Some(TemplateDirective::Custom {
						prefix,
						suffix,
						value: Some(value),
					})
				}
				_attr => None,
			}
		}
		_ => None,
	}
}
