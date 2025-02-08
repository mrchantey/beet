use crate::prelude::*;
use thiserror::Error;


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
		children: Box<Self>,
	},
	Doctype,
	Text(String),
	Comment(String),
}

type HydrationResult<T> = std::result::Result<T, HydrationError>;

impl Default for RsxTemplateNode {
	fn default() -> Self { Self::Fragment(vec![]) }
}
#[derive(Debug, Error)]
pub enum HydrationError {
	#[error("RsxNode has no tracker for {0}, ensure they are included in RstmlToRsx settings")]
	DehydrationFailed(String),
	#[error("Rusty Map is missing a tracker for {cx}\nExpected: {expected:#?}\nReceived: {received:#?}")]
	HydrationFailed {
		cx: String,
		received: RustyTracker,
		expected: Vec<RustyTracker>,
	},
	#[error("Unexpected Node\nExpected: {expected}\nReceived: {received}")]
	UnexpectedRusty {
		expected: &'static str,
		received: String,
	},
}

impl HydrationError {
	pub fn dehydration_failed(cx: &str) -> Self {
		Self::DehydrationFailed(cx.to_string())
	}

	pub fn hydration_failed(
		cx: &str,
		expected: &HashMap<RustyTracker, RsxHydratedNode>,
		received: RustyTracker,
	) -> Self {
		Self::HydrationFailed {
			cx: cx.to_string(),
			expected: expected.keys().cloned().collect(),
			received,
		}
	}
}

impl RsxTemplateNode {
	pub fn from_rsx_node(node: impl AsRef<RsxNode>) -> HydrationResult<Self> {
		match node.as_ref() {
			RsxNode::Fragment(rsx_nodes) => {
				let nodes = rsx_nodes
					.iter()
					.map(Self::from_rsx_node)
					.collect::<HydrationResult<Vec<_>>>()?;
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
				tracker: tracker.clone().ok_or_else(|| {
					HydrationError::dehydration_failed("Component")
				})?,
				tag: tag.clone(),
			}),
			RsxNode::Block(RsxBlock { effect, .. }) => {
				Ok(Self::RustBlock(effect.tracker.clone().ok_or_else(
					|| HydrationError::dehydration_failed("NodeBlock"),
				)?))
			}
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
					.collect::<HydrationResult<Vec<_>>>()?,
				children: Box::new(Self::from_rsx_node(children)?),
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
	) -> HydrationResult<RsxNode> {
		match self {
			RsxTemplateNode::Doctype => Ok(RsxNode::Doctype),
			RsxTemplateNode::Text(text) => Ok(RsxNode::Text(text)),
			RsxTemplateNode::Comment(comment) => Ok(RsxNode::Comment(comment)),
			RsxTemplateNode::Fragment(rsx_template_nodes) => {
				let nodes = rsx_template_nodes
					.into_iter()
					.map(|node| node.into_rsx_node(rusty_map))
					.collect::<HydrationResult<Vec<_>>>()?;
				Ok(RsxNode::Fragment(nodes))
			}
			RsxTemplateNode::Component {
				tracker,
				tag,
				slot_children,
			} => {
				let root =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						HydrationError::hydration_failed(
							&format!("Component: {}", tag),
							rusty_map,
							tracker,
						)
					})? {
						RsxHydratedNode::Component { root } => Ok(root),
						other => HydrationResult::Err(
							HydrationError::UnexpectedRusty {
								expected: "Component",
								received: format!("{:?}", other),
							},
						),
					}?;

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
				let (initial, register) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						HydrationError::hydration_failed(
							&format!("RustBlock"),
							rusty_map,
							tracker,
						)
					})? {
						RsxHydratedNode::RustBlock { initial, register } => {
							Ok((initial, register))
						}
						other => HydrationResult::Err(
							HydrationError::UnexpectedRusty {
								expected: "BlockNode",
								received: format!("{:?}", other),
							},
						),
					}?;
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
					.collect::<HydrationResult<Vec<_>>>()?,
				children: Box::new(children.into_rsx_node(rusty_map)?),
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
				children.clear_rusty_trackers();
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
	pub fn from_rsx_attribute(attr: &RsxAttribute) -> HydrationResult<Self> {
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
						HydrationError::DehydrationFailed(
							"AttributeValue".into(),
						)
					})?,
				})
			}
			RsxAttribute::Block { effect, .. } => {
				Ok(Self::Block(effect.tracker.clone().ok_or_else(|| {
					HydrationError::DehydrationFailed("AttributeBlock".into())
				})?))
			}
		}
	}
	/// drain the effect map into the template
	pub fn into_rsx_node(
		self,
		rusty_map: &mut HashMap<RustyTracker, RsxHydratedNode>,
	) -> HydrationResult<RsxAttribute> {
		match self {
			RsxTemplateAttribute::Key { key } => Ok(RsxAttribute::Key { key }),
			RsxTemplateAttribute::KeyValue { key, value } => {
				Ok(RsxAttribute::KeyValue { key, value })
			}
			RsxTemplateAttribute::Block(tracker) => {
				let (initial, register) = match rusty_map
					.remove(&tracker)
					.ok_or_else(|| {
						HydrationError::hydration_failed(
							"AttributeBlock",
							rusty_map,
							tracker,
						)
					})? {
					RsxHydratedNode::AttributeBlock { initial, register } => {
						Ok((initial, register))
					}
					other => {
						HydrationResult::Err(HydrationError::UnexpectedRusty {
							expected: "AttributeBlock",
							received: format!("{:?}", other),
						})
					}
				}?;

				Ok(RsxAttribute::Block {
					initial,
					effect: Effect::new(register, Some(tracker)),
				})
			}
			RsxTemplateAttribute::BlockValue { key, tracker } => {
				let (initial, register) = match rusty_map
					.remove(&tracker)
					.ok_or_else(|| {
						HydrationError::hydration_failed(
							"AttributeValue",
							rusty_map,
							tracker,
						)
					})? {
					RsxHydratedNode::AttributeValue { initial, register } => {
						Ok((initial, register))
					}
					other => {
						HydrationResult::Err(HydrationError::UnexpectedRusty {
							expected: "AttributeValue",
							received: format!("{:?}", other),
						})
					}
				}?;

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
		let root = rsx_template! { <div>{value}</div> };

		expect(&root.node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			children: Box::new(RsxTemplateNode::RustBlock(loc)),
		});
	}
	#[test]
	fn complex() {
		let ident_tracker = RustyTracker::new(0, 3802233634778759949);
		let component_tracker = RustyTracker::new(1, 3429327963174273294);
		let template = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
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
			children: Box::new(RsxTemplateNode::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: Box::new(RsxTemplateNode::Fragment(vec![
					RsxTemplateNode::Text("\n\t\t\t\t\thello ".to_string()),
					RsxTemplateNode::Component {
						tracker: component_tracker,
						tag: "MyComponent".to_string(),
						slot_children: Box::new(RsxTemplateNode::Element {
							tag: "div".to_string(),
							self_closing: false,
							attributes: vec![],
							children: Box::new(RsxTemplateNode::Text(
								"some child".to_string(),
							)),
						}),
					},
				])),
			}),
		});
	}

	#[test]
	fn ron() {
		let template = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let template_ron = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		expect(template.node).to_be(template_ron.node);
	}
}
