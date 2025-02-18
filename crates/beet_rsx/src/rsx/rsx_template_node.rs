use crate::prelude::*;
use thiserror::Error;


/// Serializable version of an rsx node that can be rehydrated.
///
/// An [RsxTemplateNode] is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateNode {
	Doctype {
		idx: RsxIdx,
	},
	Comment {
		idx: RsxIdx,
		value: String,
	},
	Text {
		idx: RsxIdx,
		value: String,
	},
	Fragment {
		idx: RsxIdx,
		items: Vec<Self>,
	},
	RustBlock {
		idx: RsxIdx,
		tracker: RustyTracker,
	},
	Element {
		idx: RsxIdx,
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Box<Self>,
	},
	/// We dont know much about components, for example when parsing
	/// a file we just get the name.
	/// The [RsxLocation] etc is is tracked by the [RustyPart::Component::root]
	Component {
		idx: RsxIdx,
		/// the hydrated part has the juicy details
		tracker: RustyTracker,
		tag: String,
		/// mapped from [RsxComponent::slot_children]
		slot_children: Box<Self>,
	},
}

pub type TemplateResult<T> = std::result::Result<T, TemplateError>;

impl Default for RsxTemplateNode {
	fn default() -> Self {
		Self::Fragment {
			idx: RsxIdx::default(),
			items: vec![],
		}
	}
}

#[derive(Debug, Error)]
pub enum TemplateError {
	#[error("RsxNode has no tracker for {0}, ensure they are included in RstmlToRsx settings")]
	DehydrationFailed(String),
	#[error("No template found for {0:?}")]
	NoTemplate(RsxLocation),
	#[error("Rusty Map is missing a tracker for {cx}\nExpected: {expected:#?}\nReceived: {received:#?}")]
	NoRustyMap {
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

impl TemplateError {
	pub fn no_rusty_map(
		cx: &str,
		expected: &HashMap<RustyTracker, RustyPart>,
		received: RustyTracker,
	) -> Self {
		Self::NoRustyMap {
			cx: cx.to_string(),
			expected: expected.keys().cloned().collect(),
			received,
		}
	}
}

impl RsxTemplateNode {
	/// get the rsx idx of the node
	pub fn rsx_idx(&self) -> RsxIdx {
		match self {
			RsxTemplateNode::Doctype { idx } => *idx,
			RsxTemplateNode::Comment { idx, .. } => *idx,
			RsxTemplateNode::Text { idx, .. } => *idx,
			RsxTemplateNode::Fragment { idx, .. } => *idx,
			RsxTemplateNode::RustBlock { idx, .. } => *idx,
			RsxTemplateNode::Element { idx, .. } => *idx,
			RsxTemplateNode::Component { idx, .. } => *idx,
		}
	}

	pub fn from_rsx_node(node: impl AsRef<RsxNode>) -> TemplateResult<Self> {
		Self::from_rsx_node_inner(node, &mut RsxIdxIncr::default())
	}
	fn from_rsx_node_inner(
		node: impl AsRef<RsxNode>,
		rsx_idx_incr: &mut RsxIdxIncr,
	) -> TemplateResult<Self> {
		let idx = rsx_idx_incr.next();
		match node.as_ref() {
			RsxNode::Fragment(rsx_nodes) => {
				let items = rsx_nodes
					.iter()
					.map(|n| Self::from_rsx_node_inner(n, rsx_idx_incr))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(Self::Fragment { idx, items })
			}
			RsxNode::Component(RsxComponent {
				tag,
				tracker,
				// ignore the node, it is the responsibility of rsx_hydrate_node
				root: _,

				slot_children,
			}) => Ok(Self::Component {
				idx,
				// location: node.location.clone(),
				// node: Box::new(Self::from_rsx_node_inner(node)?),
				slot_children: Box::new(Self::from_rsx_node_inner(
					slot_children,
					rsx_idx_incr,
				)?),
				tracker: tracker.clone(),
				tag: tag.clone(),
			}),
			RsxNode::Block(RsxBlock { effect, initial }) => {
				// even though we arent storing the initial, we sill
				// must visit to increment the idx
				let _ = Self::from_rsx_node_inner(initial, rsx_idx_incr)?;
				Ok(Self::RustBlock {
					idx,
					tracker: effect.tracker.clone(),
				})
			}
			RsxNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
			}) => Ok(Self::Element {
				tag: tag.clone(),
				idx,
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| RsxTemplateAttribute::from_rsx_attribute(attr))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(Self::from_rsx_node_inner(
					children,
					rsx_idx_incr,
				)?),
			}),
			RsxNode::Text(text) => Ok(Self::Text {
				idx,
				value: text.clone(),
			}),
			RsxNode::Comment(comment) => Ok(Self::Comment {
				idx,
				value: comment.clone(),
			}),
			RsxNode::Doctype => Ok(Self::Doctype { idx }),
		}
	}

	/// drain the effect map into an RsxNode
	/// We need the [`RsxTemplateMap`] to apply the template
	/// for nested components
	pub fn into_rsx_node(
		self,
		template_map: &RsxTemplateMap,
		rusty_map: &mut HashMap<RustyTracker, RustyPart>,
	) -> TemplateResult<RsxNode> {
		match self {
			RsxTemplateNode::Doctype { idx: _ } => Ok(RsxNode::Doctype),
			RsxTemplateNode::Text { value, idx: _ } => Ok(RsxNode::Text(value)),
			RsxTemplateNode::Comment { value, idx: _ } => {
				Ok(RsxNode::Comment(value))
			}

			RsxTemplateNode::Fragment { items, idx: _ } => {
				let nodes = items
					.into_iter()
					.map(|node| node.into_rsx_node(template_map, rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(RsxNode::Fragment(nodes))
			}
			RsxTemplateNode::Component {
				tracker,
				tag,
				slot_children,
				idx: _,
			} => {
				let root =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("Component: {}", tag),
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::Component { root } => Ok(root),
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "Component",
								received: format!("{:?}", other),
							},
						),
					}?;
				// here we need apply the template for the component
				let root = template_map.apply_template(root)?;
				Ok(RsxNode::Component(RsxComponent {
					tag: tag.clone(),
					tracker,
					root: Box::new(root),
					slot_children: Box::new(
						slot_children.into_rsx_node(template_map, rusty_map)?,
					),
				}))
			}
			RsxTemplateNode::RustBlock { tracker, idx: _ } => {
				let (initial, register) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("RustBlock"),
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::RustBlock { initial, register } => {
							Ok((initial, register))
						}
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "BlockNode",
								received: format!("{:?}", other),
							},
						),
					}?;
				Ok(RsxNode::Block(RsxBlock {
					initial: Box::new(initial),
					effect: Effect::new(register, tracker),
				}))
			}
			RsxTemplateNode::Element {
				tag,
				self_closing,
				attributes,
				children,
				idx: _,
			} => Ok(RsxNode::Element(RsxElement {
				tag,
				self_closing,
				attributes: attributes
					.into_iter()
					.map(|attr| attr.into_rsx_node(rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(
					children.into_rsx_node(template_map, rusty_map)?,
				),
			})),
		}
	}

	/// A simple dfs visitor for an rsx template node
	pub fn visit(&self, func: impl Fn(&Self)) { self.visit_inner(&func); }
	fn visit_inner(&self, func: &impl Fn(&Self)) {
		func(self);
		match self {
			RsxTemplateNode::Fragment { items, .. } => {
				for item in items {
					item.visit(func);
				}
			}
			RsxTemplateNode::Component { slot_children, .. } => {
				slot_children.visit(func);
			}
			RsxTemplateNode::Element { children, .. } => {
				children.visit(func);
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
	pub fn from_rsx_attribute(attr: &RsxAttribute) -> TemplateResult<Self> {
		match attr {
			RsxAttribute::Key { key } => Ok(Self::Key { key: key.clone() }),
			RsxAttribute::KeyValue { key, value } => Ok(Self::KeyValue {
				key: key.clone(),
				value: value.clone(),
			}),
			RsxAttribute::BlockValue { key, effect, .. } => {
				Ok(Self::BlockValue {
					key: key.clone(),
					tracker: effect.tracker,
				})
			}
			RsxAttribute::Block { effect, .. } => {
				Ok(Self::Block(effect.tracker))
			}
		}
	}
	/// drain the rusty map into the template
	pub fn into_rsx_node(
		self,
		rusty_map: &mut HashMap<RustyTracker, RustyPart>,
	) -> TemplateResult<RsxAttribute> {
		match self {
			RsxTemplateAttribute::Key { key } => Ok(RsxAttribute::Key { key }),
			RsxTemplateAttribute::KeyValue { key, value } => {
				Ok(RsxAttribute::KeyValue { key, value })
			}
			RsxTemplateAttribute::Block(tracker) => {
				let (initial, register) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							"AttributeBlock",
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::AttributeBlock { initial, register } => {
							Ok((initial, register))
						}
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "AttributeBlock",
								received: format!("{:?}", other),
							},
						),
					}?;

				Ok(RsxAttribute::Block {
					initial,
					effect: Effect::new(register, tracker),
				})
			}
			RsxTemplateAttribute::BlockValue { key, tracker } => {
				let (initial, register) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							"AttributeValue",
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::AttributeValue { initial, register } => {
							Ok((initial, register))
						}
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "AttributeValue",
								received: format!("{:?}", other),
							},
						),
					}?;

				Ok(RsxAttribute::BlockValue {
					key,
					initial,
					effect: Effect::new(register, tracker),
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
		let tracker = RustyTracker::new(0, 15046980652419922415);
		let root = rsx_template! { <div>{value}</div> };

		expect(&root.node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			idx: 0,
			attributes: vec![],
			children: Box::new(RsxTemplateNode::RustBlock { tracker, idx: 1 }),
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
			idx: 0,
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
				idx: 1,
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: Box::new(RsxTemplateNode::Fragment {
					idx: 2,
					items: vec![
						RsxTemplateNode::Text {
							idx: 3,
							value: "\n\t\t\t\t\thello ".to_string(),
						},
						RsxTemplateNode::Component {
							idx: 4,
							tracker: component_tracker,
							tag: "MyComponent".to_string(),
							slot_children: Box::new(RsxTemplateNode::Element {
								idx: 5,
								tag: "div".to_string(),
								self_closing: false,
								attributes: vec![],
								children: Box::new(RsxTemplateNode::Text {
									idx: 6,
									value: "some child".to_string(),
								}),
							}),
						},
					],
				}),
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
