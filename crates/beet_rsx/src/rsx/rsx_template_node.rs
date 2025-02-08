use crate::prelude::*;
use anyhow::Result;
use std::collections::HashMap;



/// Serializable version of an rsx node that can be rehydrated.
///
/// This has absolute symmetry with [RsxNode] but with each rusty bit
/// replaced by [RustyTracker].
///
/// An [RsxTemplateNode] is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateNode {
	Fragment(Vec<Self>),
	/// We dont know much about components, for example when parsing
	/// a file we just get the name.
	/// The [RsxLocation] etc is is tracked by the [RsxHydratedNode::Component::root]
	Component {
		/// the hydrated part has the juicy details
		tracker: RustyTracker,
		tag: String,
		/// mapped from [RsxComponent::slot_children]
		slot_children: Box<Self>,
	},
	RustBlock(RustyTracker),
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

impl Default for RsxTemplateNode {
	fn default() -> Self { Self::Fragment(vec![]) }
}

fn from_node_tracker_error(cx: &str) -> anyhow::Error {
	anyhow::anyhow!("Template Builder - RsxNode has no tracker for {cx}, ensure they are included in RstmlToRsx settings")
}
fn to_node_tracker_error(
	rusty_map: &HashMap<RustyTracker, RsxHydratedNode>,
	tracker: &RustyTracker,
	cx: &str,
) -> anyhow::Error {
	anyhow::anyhow!("Template Builder - Rusty Map is missing a tracker for {cx}\nExpected: {:#?}\nReceived: {:#?}",tracker,rusty_map.keys())
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
			RsxNode::Component(RsxComponent {
				tag,
				tracker,
				// ignore the node, it is the responsibility of rsx_hydrate_node
				root: _,

				slot_children,
			}) => Ok(Self::Component {
				// location: node.location.clone(),
				// node: Box::new(Self::from_rsx_node(node)?),
				slot_children: Box::new(Self::from_rsx_node(slot_children)?),
				tracker: tracker
					.clone()
					.ok_or_else(|| from_node_tracker_error("Component"))?,
				tag: tag.clone(),
			}),
			RsxNode::Block(RsxBlock { effect, .. }) => Ok(Self::RustBlock(
				effect
					.tracker
					.clone()
					.ok_or_else(|| from_node_tracker_error("NodeBlock"))?,
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
		// incorrect! we need a map for each component
		rusty_map: &mut HashMap<RustyTracker, RsxHydratedNode>,
	) -> Result<RsxNode> {
		match self {
			RsxTemplateNode::Doctype => Ok(RsxNode::Doctype),
			RsxTemplateNode::Text(text) => Ok(RsxNode::Text(text)),
			RsxTemplateNode::Comment(comment) => Ok(RsxNode::Comment(comment)),
			RsxTemplateNode::Fragment(rsx_template_nodes) => {
				let nodes = rsx_template_nodes
					.into_iter()
					.map(|node| node.into_rsx_node(rusty_map))
					.collect::<Result<Vec<_>>>()?;
				Ok(RsxNode::Fragment(nodes))
			}
			RsxTemplateNode::Component {
				tracker,
				tag,
				slot_children,
			} => {
				let RsxHydratedNode::Component { root } =
					rusty_map.remove(&tracker).ok_or_else(|| {
						to_node_tracker_error(
							rusty_map,
							&tracker,
							&format!("Component: {}", &tag),
						)
					})?
				else {
					anyhow::bail!("expected Component")
				};
				Ok(RsxNode::Component(RsxComponent {
					tag: tag.clone(),
					tracker: Some(tracker),
					root: Box::new(root),
					slot_children: Box::new(
						slot_children.into_rsx_node(rusty_map)?,
					),
				}))
			}
			RsxTemplateNode::RustBlock(tracker) => {
				let RsxHydratedNode::RustBlock { initial, register } =
					rusty_map.remove(&tracker).ok_or_else(|| {
						to_node_tracker_error(rusty_map, &tracker, "RustBlock")
					})?
				else {
					anyhow::bail!("expected Rust Block")
				};
				Ok(RsxNode::Block(RsxBlock {
					initial: Box::new(initial),
					effect: Effect::new(register, Some(tracker)),
				}))
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
					.map(|attr| attr.into_rsx_node(rusty_map))
					.collect::<Result<Vec<_>>>()?,
				children: children
					.into_iter()
					.map(|node| node.into_rsx_node(rusty_map))
					.collect::<Result<Vec<_>>>()?,
			})),
		}
	}

	/// allow two templates to be compared without considering line and column
	#[cfg(test)]
	#[deprecated = "from linecol locations"]
	pub fn clear_rusty_trackers(&mut self) {
		match self {
			RsxTemplateNode::Component { tracker, .. } => {
				tracker.clear();
			}
			RsxTemplateNode::Fragment(children) => {
				for child in children {
					child.clear_rusty_trackers();
				}
			}
			RsxTemplateNode::RustBlock(tracker) => {
				tracker.clear();
			}
			RsxTemplateNode::Element {
				attributes,
				children,
				..
			} => {
				for attr in attributes {
					if let RsxTemplateAttribute::BlockValue {
						tracker, ..
					} = attr
					{
						tracker.clear();
					}
					if let RsxTemplateAttribute::Block(tracker) = attr {
						tracker.clear();
					}
				}
				// Recursively process children
				for child in children {
					child.clear_rusty_trackers();
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
	Block(RustyTracker),
	BlockValue { key: String, tracker: RustyTracker },
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
					tracker: effect.tracker.clone().ok_or_else(|| {
						from_node_tracker_error("AttributeValue")
					})?,
				})
			}
			RsxAttribute::Block { effect, .. } => {
				Ok(Self::Block(effect.tracker.clone().ok_or_else(|| {
					from_node_tracker_error("AttributeBlock")
				})?))
			}
		}
	}
	/// drain the effect map into the template
	pub fn into_rsx_node(
		self,
		rusty_map: &mut HashMap<RustyTracker, RsxHydratedNode>,
	) -> Result<RsxAttribute> {
		match self {
			RsxTemplateAttribute::Key { key } => Ok(RsxAttribute::Key { key }),
			RsxTemplateAttribute::KeyValue { key, value } => {
				Ok(RsxAttribute::KeyValue { key, value })
			}
			RsxTemplateAttribute::Block(tracker) => {
				let RsxHydratedNode::AttributeBlock { initial, register } =
					rusty_map.remove(&tracker).ok_or_else(|| {
						to_node_tracker_error(
							rusty_map,
							&tracker,
							"AttributeBlock",
						)
					})?
				else {
					anyhow::bail!("expected Attribute Block")
				};
				Ok(RsxAttribute::Block {
					initial,
					effect: Effect::new(register, Some(tracker)),
				})
			}
			RsxTemplateAttribute::BlockValue { key, tracker } => {
				let RsxHydratedNode::AttributeValue { initial, register } =
					rusty_map.remove(&tracker).ok_or_else(|| {
						to_node_tracker_error(
							rusty_map,
							&tracker,
							"AttributeValue",
						)
					})?
				else {
					anyhow::bail!("expected Attribute Block")
				};
				Ok(RsxAttribute::BlockValue {
					key,
					initial,
					effect: Effect::new(register, Some(tracker)),
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
		let loc = RustyTracker::new(0, 15046980652419922415);
		let root = rsx_template! {<div>{value}</div>};

		expect(&root.node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			children: vec![RsxTemplateNode::RustBlock(loc)],
		});
	}
	#[test]
	fn complex() {
		let ident_tracker = RustyTracker::new(1, 3802233634778759949);
		let component_tracker = RustyTracker::new(0, 3429327963174273294);
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

		expect(&template.node).to_be(&RsxTemplateNode::Element {
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
					tracker: ident_tracker,
				},
			],
			children: vec![RsxTemplateNode::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: vec![
					RsxTemplateNode::Text("hello\n\t\t\t\t\t".to_string()),
					RsxTemplateNode::Component {
						tracker: component_tracker,
						tag: "MyComponent".to_string(),
						slot_children: Box::new(RsxTemplateNode::Element {
							tag: "div".to_string(),
							self_closing: false,
							attributes: vec![],
							children: vec![RsxTemplateNode::Text(
								"some child".to_string(),
							)],
						}),
					},
				],
			}],
		});
	}

	#[test]
	fn ron() {
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
		let template_ron = rsx_template! {
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
		expect(template.node).to_be(template_ron.node);
	}
}
