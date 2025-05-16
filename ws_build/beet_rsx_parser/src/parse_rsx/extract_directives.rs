use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use sweet::prelude::Pipeline;
use sweet::prelude::PipelineTarget;


/// For each [`ElementTokens`], read its [`attributes`](ElementTokens::attributes) and extract them
/// into the [`directives`](ElementTokens::directives) field.
#[derive(Default)]
pub struct ExtractTemplateDirectives;

impl<T: ElementTokensVisitor> Pipeline<T, Result<T>>
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
) -> Result<()> {
	let mut result = Ok(());
	attributes.retain(|attr| match attr_to_template_directive(attr) {
		Ok(Some(directive)) => {
			meta.push_directive(directive);
			false
		}
		Err(e) => {
			result = Err(e);
			true
		}
		Ok(None) => true,
	});
	result
}

fn attr_to_template_directive(
	attr: &AttributeTokens,
) -> Result<Option<TemplateDirective>> {
	match attr {
		AttributeTokens::Key { key } => {
			TemplateDirective::try_from_attr(key.as_str(), None)?
		}
		AttributeTokens::KeyValueLit { key, value } => {
			let value = AttributeTokens::lit_to_string(&value);
			TemplateDirective::try_from_attr(key.as_str(), Some(&value))?
		}
		_ => None,
	}
	.xok()
}
