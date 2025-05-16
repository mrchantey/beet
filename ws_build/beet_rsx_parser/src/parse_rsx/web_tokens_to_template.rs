use crate::prelude::*;
use beet_common::prelude::*;
use sweet::prelude::*;

/// Convert [`WebTokens`] to a ron format.
/// Rust block token streams will be hashed by [Span::start]
#[derive(Debug, Default)]
pub struct WebTokensToTemplate;

impl Pipeline<WebTokens, WebNodeTemplate> for WebTokensToTemplate {
	fn apply(mut self, node: WebTokens) -> WebNodeTemplate {
		self.map_node(node)
	}
}

impl WebTokensToTemplate {
	/// returns an WebNodeTemplate
	pub fn map_node(&mut self, node: WebTokens) -> WebNodeTemplate {
		match node {
			WebTokens::Fragment { nodes, meta } => {
				let nodes = nodes.into_iter().map(|n| self.map_node(n));
				WebNodeTemplate::Fragment {
					items: nodes.collect(),
					meta,
				}
			}
			WebTokens::Doctype { value: _, meta } => {
				WebNodeTemplate::Doctype { meta }
			}
			WebTokens::Comment { value, meta } => WebNodeTemplate::Comment {
				value: value.to_string(),
				meta,
			},
			WebTokens::Text { value, meta } => WebNodeTemplate::Text {
				value: value.to_string(),
				meta,
			},
			WebTokens::Block {
				value: _,
				meta,
				tracker,
			} => WebNodeTemplate::RustBlock { tracker, meta },
			WebTokens::Element {
				component,
				children,
				self_closing,
			} => WebNodeTemplate::Element {
				tag: component.tag.to_string(),
				self_closing,
				attributes: component
					.attributes
					.into_iter()
					.map(|a| self.map_attribute(a))
					.collect(),
				children: Box::new(self.map_node(*children)),
				meta: component.meta,
			},
			WebTokens::Component {
				component,
				children,
				tracker,
			} => WebNodeTemplate::Component {
				tag: component.tag.to_string(),
				slot_children: Box::new(self.map_node(*children)),
				tracker,
				meta: component.meta,
			},
		}
	}


	fn map_attribute(&mut self, attr: AttributeTokens) -> RsxTemplateAttribute {
		match attr {
			AttributeTokens::Block { block: _, tracker } => {
				RsxTemplateAttribute::Block(tracker)
			}
			AttributeTokens::Key { key } => RsxTemplateAttribute::Key {
				key: key.to_string(),
			},
			AttributeTokens::KeyValueLit { key, value } => {
				// ron stringifies all lit values?
				// tbh not sure why we need to do this but it complains need string
				let value = AttributeTokens::lit_to_string(&value);
				RsxTemplateAttribute::KeyValue {
					key: key.to_string(),
					value,
				}
			}
			AttributeTokens::KeyValueExpr {
				key,
				tracker,
				value: _,
			} => RsxTemplateAttribute::BlockValue {
				key: key.to_string(),
				tracker,
			},
		}
	}
}
