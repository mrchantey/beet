use crate::prelude::*;
use anyhow::Result;
use std::collections::HashMap;
/// An rsx template is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateNode {
	Fragment(Vec<Self>),
	// only used by the rsx_template! macro,
	// components are already collapsed when traversing [RsxNode]
	Component {
		loc: LineColumn,
		tag: String,
	},
	RustBlock(LineColumn),
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Vec<Self>,
	},
	Doctype,
	Text(String),
	Comment(String),
}
fn no_location() -> anyhow::Error {
	anyhow::anyhow!("effect has no location, ensure they are collected")
}

impl RsxTemplateNode {
	pub fn from_rsx_node(node: impl AsRef<RsxNode>) -> Result<Self> {
		match node.as_ref() {
			RsxNode::Fragment(rsx_nodes) => {
				let nodes = rsx_nodes
					.iter()
					.map(Self::from_rsx_node)
					.collect::<Result<Vec<_>>>()?;
				Ok(Self::Fragment(nodes))
			}
			RsxNode::Component { tag, loc, .. } => Ok(Self::Component {
				loc: loc.clone().ok_or_else(no_location)?,
				tag: tag.clone(),
			}),
			RsxNode::Block { effect, .. } => Ok(Self::RustBlock(
				effect.location.clone().ok_or_else(no_location)?,
			)),
			RsxNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
			}) => Ok(Self::Element {
				tag: tag.clone(),
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| RsxTemplateAttribute::from_rsx_attribute(attr))
					.collect::<Result<Vec<_>>>()?,
				children: (children
					.iter()
					.map(Self::from_rsx_node)
					.collect::<Result<Vec<_>>>()?),
			}),
			RsxNode::Text(text) => Ok(Self::Text(text.clone())),
			RsxNode::Comment(comment) => Ok(Self::Comment(comment.clone())),
			RsxNode::Doctype => Ok(Self::Doctype),
		}
	}

	/// drain the effect map into an RsxNode
	pub fn into_rsx_node(
		self,
		effect_map: &mut HashMap<LineColumn, RsxHydratedNode>,
	) -> Result<RsxNode> {
		match self {
			RsxTemplateNode::Doctype => Ok(RsxNode::Doctype),
			RsxTemplateNode::Text(text) => Ok(RsxNode::Text(text)),
			RsxTemplateNode::Comment(comment) => Ok(RsxNode::Comment(comment)),
			RsxTemplateNode::Fragment(rsx_template_nodes) => {
				let nodes = rsx_template_nodes
					.into_iter()
					.map(|node| node.into_rsx_node(effect_map))
					.collect::<Result<Vec<_>>>()?;
				Ok(RsxNode::Fragment(nodes))
			}
			RsxTemplateNode::Component { loc, tag, .. } => {
				let RsxHydratedNode::Component { node } =
					effect_map.remove(&loc).ok_or_else(no_location)?
				else {
					anyhow::bail!("expected Component")
				};
				Ok(RsxNode::Component {
					tag: tag.clone(),
					loc: Some(loc),
					node: Box::new(node),
				})
			}
			RsxTemplateNode::RustBlock(line_column) => {
				let RsxHydratedNode::RustBlock { initial, register } =
					effect_map.remove(&line_column).ok_or_else(no_location)?
				else {
					anyhow::bail!("expected Rust Block")
				};
				Ok(RsxNode::Block {
					initial: Box::new(initial),
					effect: Effect::new(register, Some(line_column)),
				})
			}
			RsxTemplateNode::Element {
				tag,
				self_closing,
				attributes,
				children,
			} => Ok(RsxNode::Element(RsxElement {
				tag,
				self_closing,
				attributes: attributes
					.into_iter()
					.map(|attr| attr.into_rsx_node(effect_map))
					.collect::<Result<Vec<_>>>()?,
				children: children
					.into_iter()
					.map(|node| node.into_rsx_node(effect_map))
					.collect::<Result<Vec<_>>>()?,
			})),
		}
	}

	/// allow two templates to be compared without considering line and column
	#[cfg(test)]
	pub fn zero_out_linecol(&mut self) {
		match self {
			RsxTemplateNode::Component { loc: hash, .. } => {
				hash.line = 0;
				hash.column = 0;
			}
			RsxTemplateNode::Fragment(children) => {
				for child in children {
					child.zero_out_linecol();
				}
			}
			RsxTemplateNode::RustBlock(loc) => {
				loc.line = 0;
				loc.column = 0;
			}
			RsxTemplateNode::Element {
				attributes,
				children,
				..
			} => {
				for attr in attributes {
					if let RsxTemplateAttribute::BlockValue { value, .. } = attr
					{
						value.line = 0;
						value.column = 0;
					}
					if let RsxTemplateAttribute::Block(loc) = attr {
						loc.line = 0;
						loc.column = 0;
					}
				}
				// Recursively process children
				for child in children {
					child.zero_out_linecol();
				}
			}
			_ => {}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateAttribute {
	Key { key: String },
	KeyValue { key: String, value: String },
	Block(LineColumn),
	BlockValue { key: String, value: LineColumn },
}

impl RsxTemplateAttribute {
	pub fn from_rsx_attribute(attr: &RsxAttribute) -> Result<Self> {
		match attr {
			RsxAttribute::Key { key } => Ok(Self::Key { key: key.clone() }),
			RsxAttribute::KeyValue { key, value } => Ok(Self::KeyValue {
				key: key.clone(),
				value: value.clone(),
			}),
			RsxAttribute::BlockValue { key, effect, .. } => {
				Ok(Self::BlockValue {
					key: key.clone(),
					value: effect.location.clone().ok_or_else(no_location)?,
				})
			}
			RsxAttribute::Block { effect, .. } => Ok(Self::Block(
				effect.location.clone().ok_or_else(no_location)?,
			)),
		}
	}
	/// drain the effect map into the template
	pub fn into_rsx_node(
		self,
		effect_map: &mut HashMap<LineColumn, RsxHydratedNode>,
	) -> Result<RsxAttribute> {
		match self {
			RsxTemplateAttribute::Key { key } => Ok(RsxAttribute::Key { key }),
			RsxTemplateAttribute::KeyValue { key, value } => {
				Ok(RsxAttribute::KeyValue { key, value })
			}
			RsxTemplateAttribute::Block(line_column) => {
				let RsxHydratedNode::AttributeBlock { initial, register } =
					effect_map.remove(&line_column).ok_or_else(no_location)?
				else {
					anyhow::bail!("expected Attribute Block")
				};
				Ok(RsxAttribute::Block {
					initial,
					effect: Effect::new(register, Some(line_column)),
				})
			}
			RsxTemplateAttribute::BlockValue { key, value } => {
				let RsxHydratedNode::AttributeValue { initial, register } =
					effect_map.remove(&value).ok_or_else(no_location)?
				else {
					anyhow::bail!("expected Attribute Block")
				};
				Ok(RsxAttribute::BlockValue {
					key,
					initial,
					effect: Effect::new(register, Some(value)),
				})
			}
		}
	}
}


/// TODO this may be used for resumability
#[allow(dead_code)]
struct RsxTemplateNodeToHtml {
	/// The attribute to identify the block,
	/// ie `<div>{rust_code}</div>`
	/// will become `<div><rsx-block hash="1234"/></div>`
	rust_block_tag: String,
	/// An attribute to identify a rust block attribute,
	/// ie `<div {rust_code}/>`
	/// will become `<div rsx-attr-block="1234"/>`
	attribute_block_key: String,
	/// An attribute to identify a rust block attribute value,
	/// ie `<div key={rust_code}/>`
	/// will become `<div key="rsx-attr-value-1234"/>`
	attribute_value_prefix: String,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		let loc = LineColumn::new(line!() + 1, 33);
		let node = rsx_template! {<div>{value}</div>};

		expect(&node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			children: vec![RsxTemplateNode::RustBlock(loc)],
		});
	}
	#[test]
	fn complex() {
		let ident_linecol = LineColumn::new(line!() + 7, 10);
		let component_linecol = LineColumn::new(line!() + 9, 5);
		let template = rsx_template! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				>
				<p>hello
					<MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		expect(&template).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![
				RsxTemplateAttribute::Key {
					key: "key".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "str".to_string(),
					value: "value".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "num".to_string(),
					value: "32".to_string(),
				},
				RsxTemplateAttribute::BlockValue {
					key: "ident".to_string(),
					value: ident_linecol,
				},
			],
			children: vec![RsxTemplateNode::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: vec![
					RsxTemplateNode::Text("hello\n\t\t\t\t\t".to_string()),
					RsxTemplateNode::Component {
						loc: component_linecol,
						tag: "MyComponent".to_string(),
					},
				],
			}],
		});
	}

	#[test]
	fn ron() {
		let mut template = rsx_template! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				>
				<p>hello
					<MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let template_ron = rsx_template_ron! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				>
				<p>hello
					<MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		let mut parsed =
			ron::de::from_str::<RsxTemplateNode>(&template_ron).unwrap();
		template.zero_out_linecol();
		parsed.zero_out_linecol();

		expect(template).to_be(parsed);
	}
}
