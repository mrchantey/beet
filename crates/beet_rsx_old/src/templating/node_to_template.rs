use crate::prelude::*;
use beet_common::prelude::*;

pub struct NodeToTemplate;

impl<T: AsRef<WebNode>> Pipeline<T, TemplateResult<WebNodeTemplate>>
	for NodeToTemplate
{
	fn apply(self, node: T) -> TemplateResult<WebNodeTemplate> {
		match node.as_ref() {
			WebNode::Fragment(RsxFragment { nodes, meta }) => {
				let items = nodes
					.iter()
					.map(|n| n.xpipe(NodeToTemplate))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(WebNodeTemplate::Fragment {
					items,
					meta: meta.clone(),
				})
			}
			WebNode::Component(RsxComponent {
				tag,
				tracker,
				// ignore root, its a seperate tree
				node: _,
				// type_name cannot be statically changed
				type_name: _,
				// ron cannot be statically generated
				ron: _,
				slot_children,
				meta,
			}) => Ok(WebNodeTemplate::Component {
				slot_children: Box::new(slot_children.xpipe(NodeToTemplate)?),
				tracker: tracker.clone(),
				tag: tag.clone(),
				meta: meta.clone(),
			}),
			WebNode::Block(RsxBlock {
				effect,
				// ignore initial, its a seperate tree
				initial: _,
				meta,
			}) => Ok(WebNodeTemplate::RustBlock {
				tracker: effect.tracker.clone(),
				meta: meta.clone(),
			}),
			WebNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
				meta,
			}) => Ok(WebNodeTemplate::Element {
				tag: tag.clone(),
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| attr_to_template(attr))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(children.xpipe(NodeToTemplate)?),
				meta: meta.clone(),
			}),
			WebNode::Text(RsxText { value, meta }) => {
				Ok(WebNodeTemplate::Text {
					value: value.clone(),
					meta: meta.clone(),
				})
			}
			WebNode::Comment(RsxComment { value, meta }) => {
				Ok(WebNodeTemplate::Comment {
					value: value.clone(),
					meta: meta.clone(),
				})
			}
			WebNode::Doctype(RsxDoctype { meta }) => {
				Ok(WebNodeTemplate::Doctype { meta: meta.clone() })
			}
		}
	}
}

fn attr_to_template(
	attr: &RsxAttribute,
) -> TemplateResult<RsxTemplateAttribute> {
	match attr {
		RsxAttribute::Key { key } => {
			Ok(RsxTemplateAttribute::Key { key: key.clone() })
		}
		RsxAttribute::KeyValue { key, value } => {
			Ok(RsxTemplateAttribute::KeyValue {
				key: key.clone(),
				value: value.clone(),
			})
		}
		RsxAttribute::BlockValue { key, effect, .. } => {
			Ok(RsxTemplateAttribute::BlockValue {
				key: key.clone(),
				tracker: effect.tracker,
			})
		}
		RsxAttribute::Block { effect, .. } => {
			Ok(RsxTemplateAttribute::Block(effect.tracker))
		}
	}
}
