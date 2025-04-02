use crate::prelude::*;


pub struct NodeToTemplate;

impl<T: AsRef<RsxNode>> RsxPipeline<T, TemplateResult<RsxTemplateNode>>
	for NodeToTemplate
{
	fn apply(self, node: T) -> TemplateResult<RsxTemplateNode> {
		match node.as_ref() {
			RsxNode::Fragment(RsxFragment { nodes, meta }) => {
				let items = nodes
					.iter()
					.map(|n| n.bpipe(NodeToTemplate))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(RsxTemplateNode::Fragment {
					items,
					meta: meta.clone(),
				})
			}
			RsxNode::Component(RsxComponent {
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
			}) => Ok(RsxTemplateNode::Component {
				slot_children: Box::new(slot_children.bpipe(NodeToTemplate)?),
				tracker: tracker.clone(),
				tag: tag.clone(),
				meta: meta.clone(),
			}),
			RsxNode::Block(RsxBlock {
				effect,
				// ignore initial, its a seperate tree
				initial: _,
				meta,
			}) => Ok(RsxTemplateNode::RustBlock {
				tracker: effect.tracker.clone(),
				meta: meta.clone(),
			}),
			RsxNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
				meta,
			}) => Ok(RsxTemplateNode::Element {
				tag: tag.clone(),
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| attr_to_template(attr))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(children.bpipe(NodeToTemplate)?),
				meta: meta.clone(),
			}),
			RsxNode::Text(RsxText { value, meta }) => {
				Ok(RsxTemplateNode::Text {
					value: value.clone(),
					meta: meta.clone(),
				})
			}
			RsxNode::Comment(RsxComment { value, meta }) => {
				Ok(RsxTemplateNode::Comment {
					value: value.clone(),
					meta: meta.clone(),
				})
			}
			RsxNode::Doctype(RsxDoctype { meta }) => {
				Ok(RsxTemplateNode::Doctype { meta: meta.clone() })
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
