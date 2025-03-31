use crate::prelude::*;


pub struct NodeToTemplate;

impl<T: AsRef<RsxNode>> RsxPipeline<T, TemplateResult<RsxTemplateNode>>
	for NodeToTemplate
{
	fn apply(self, node: T) -> TemplateResult<RsxTemplateNode> {
		match node.as_ref() {
			RsxNode::Fragment(RsxFragment { nodes, location }) => {
				let items = nodes
					.iter()
					.map(|n| n.bpipe(NodeToTemplate))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(RsxTemplateNode::Fragment {
					items,
					location: location.clone(),
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
				// not sure if we need to serialize these
				template_directives,
				slot_children,
				location,
			}) => Ok(RsxTemplateNode::Component {
				slot_children: Box::new(slot_children.bpipe(NodeToTemplate)?),
				tracker: tracker.clone(),
				tag: tag.clone(),
				template_directives: template_directives.clone(),
				location: location.clone(),
			}),
			RsxNode::Block(RsxBlock {
				effect,
				// ignore initial, its a seperate tree
				initial: _,
				location,
			}) => Ok(RsxTemplateNode::RustBlock {
				tracker: effect.tracker.clone(),
				location: location.clone(),
			}),
			RsxNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
				location,
			}) => Ok(RsxTemplateNode::Element {
				tag: tag.clone(),
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| attr_to_template(attr))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(children.bpipe(NodeToTemplate)?),
				location: location.clone(),
			}),
			RsxNode::Text(RsxText { value, location }) => {
				Ok(RsxTemplateNode::Text {
					value: value.clone(),
					location: location.clone(),
				})
			}
			RsxNode::Comment(RsxComment { value, location }) => {
				Ok(RsxTemplateNode::Comment {
					value: value.clone(),
					location: location.clone(),
				})
			}
			RsxNode::Doctype(RsxDoctype { location }) => {
				Ok(RsxTemplateNode::Doctype {
					location: location.clone(),
				})
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
