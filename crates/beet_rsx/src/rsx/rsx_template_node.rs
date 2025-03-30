use crate::prelude::*;
use thiserror::Error;


/// Serializable version of an rsx node that can be rehydrated.
///
/// An [RsxTemplateNode] is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
///
/// Templates do not recurse into rusty parts,
/// ie [`RsxBlock::initial`] or [`RsxComponent::node`] are not recursed into.
/// For this reason its important that the [`RsxTemplateMap`] visits these
/// children when applying the templates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateNode {
	/// Serializable [`RsxNode::Doctype`]
	Doctype { location: Option<RsxMacroLocation> },
	/// Serializable [`RsxNode::Comment`]
	Comment {
		value: String,
		location: Option<RsxMacroLocation>,
	},
	/// Serializable [`RsxNode::Text`]
	Text {
		value: String,
		location: Option<RsxMacroLocation>,
	},
	/// Serializable [`RsxNode::Fragment`]
	Fragment {
		items: Vec<Self>,
		location: Option<RsxMacroLocation>,
	},
	/// Serializable [`RsxNode::Block`]
	/// the initial value is the responsibility of the [RustyPart::RustBlock]
	RustBlock {
		tracker: RustyTracker,
		location: Option<RsxMacroLocation>,
	},
	/// Serializable [`RsxNode::Element`]
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Box<Self>,
		location: Option<RsxMacroLocation>,
	},
	/// Serializable [`RsxNode::Component`]
	/// We dont know much about components, for example when parsing
	/// a file we just get the name.
	/// The [RsxMacroLocation] etc is is tracked by the [RustyPart::Component::root]
	Component {
		/// the hydrated part has the juicy details
		tracker: RustyTracker,
		tag: String,
		/// mapped from [RsxComponent::slot_children]
		slot_children: Box<Self>,
		template_directives: Vec<TemplateDirective>,
		location: Option<RsxMacroLocation>,
	},
}

pub type TemplateResult<T> = std::result::Result<T, TemplateError>;

impl Default for RsxTemplateNode {
	fn default() -> Self {
		Self::Fragment {
			items: Default::default(),
			location: None,
		}
	}
}

#[derive(Debug, Error)]
pub enum TemplateError {
	#[error(
		"RsxNode has no tracker for {0}, ensure they are included in RstmlToRsx settings"
	)]
	DehydrationFailed(String),
	#[error(
		"No template found\nExpected: {expected:#?}\nReceived: {received:#?}"
	)]
	NoTemplate {
		expected: RsxMacroLocation,
		received: Vec<RsxMacroLocation>,
	},
	#[error(
		"Rusty Map is missing a tracker for {cx}\nExpected: {expected:#?}\nReceived: {received:#?}"
	)]
	NoRustyMap {
		cx: String,
		expected: RustyTracker,
		received: Vec<RustyTracker>,
	},
	#[error("Unexpected Node\nExpected: {expected}\nReceived: {received}")]
	UnexpectedRusty {
		expected: &'static str,
		received: String,
	},
	#[error("Location: {location:#?}\nError: {err}")]
	WithLocation {
		location: RsxMacroLocation,
		err: Box<Self>,
	},
}

impl TemplateError {
	pub fn with_location(self, location: RsxMacroLocation) -> Self {
		Self::WithLocation {
			location,
			err: Box::new(self),
		}
	}

	pub fn no_rusty_map(
		cx: &str,
		received_map: &HashMap<RustyTracker, RustyPart>,
		expected: RustyTracker,
	) -> Self {
		Self::NoRustyMap {
			cx: cx.to_string(),
			received: received_map.keys().cloned().collect(),
			expected,
		}
	}
}

impl RsxTemplateNode {
	#[cfg(feature = "serde")]
	pub fn from_ron(ron: &str) -> anyhow::Result<Self> {
		ron::de::from_str(ron).map_err(Into::into)
	}

	pub fn location(&self) -> Option<&RsxMacroLocation> {
		match self {
			RsxTemplateNode::Doctype { location }
			| RsxTemplateNode::Comment { location, .. }
			| RsxTemplateNode::Text { location, .. }
			| RsxTemplateNode::Fragment { location, .. }
			| RsxTemplateNode::RustBlock { location, .. }
			| RsxTemplateNode::Element { location, .. }
			| RsxTemplateNode::Component { location, .. } => location.as_ref(),
		}
	}

	pub fn from_rsx_node(node: impl AsRef<RsxNode>) -> TemplateResult<Self> {
		match node.as_ref() {
			RsxNode::Fragment(RsxFragment { nodes, location }) => {
				let items = nodes
					.iter()
					.map(|n| Self::from_rsx_node(n))
					.collect::<TemplateResult<Vec<_>>>()?;
				Ok(Self::Fragment {
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
			}) => Ok(Self::Component {
				// location: node.location.clone(),
				// node: Box::new(Self::from_rsx_node(node)?),
				slot_children: Box::new(Self::from_rsx_node(slot_children)?),
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
			}) => Ok(Self::RustBlock {
				tracker: effect.tracker.clone(),
				location: location.clone(),
			}),
			RsxNode::Element(RsxElement {
				tag,
				attributes,
				children,
				self_closing,
				location,
			}) => Ok(Self::Element {
				tag: tag.clone(),
				self_closing: *self_closing,
				attributes: attributes
					.iter()
					.map(|attr| RsxTemplateAttribute::from_rsx_attribute(attr))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(Self::from_rsx_node(children)?),
				location: location.clone(),
			}),
			RsxNode::Text(RsxText { value, location }) => Ok(Self::Text {
				value: value.clone(),
				location: location.clone(),
			}),
			RsxNode::Comment(RsxComment { value, location }) => {
				Ok(Self::Comment {
					value: value.clone(),
					location: location.clone(),
				})
			}
			RsxNode::Doctype(RsxDoctype { location }) => Ok(Self::Doctype {
				location: location.clone(),
			}),
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
		let node: RsxNode = match self {
			RsxTemplateNode::Doctype { location } => {
				RsxDoctype { location }.into()
			}
			RsxTemplateNode::Text { value, location } => {
				RsxText { value, location }.into()
			}
			RsxTemplateNode::Comment { value, location } => {
				RsxComment { value, location }.into()
			}

			RsxTemplateNode::Fragment { items, location } => {
				let nodes = items
					.into_iter()
					.map(|node| node.into_rsx_node(template_map, rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?;
				RsxFragment { nodes, location }.into()
			}
			RsxTemplateNode::Component {
				tracker,
				tag,
				slot_children,
				template_directives,
				location,
			} => {
				let (root, type_name, ron) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("Component: {}", tag),
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::Component {
							root,
							type_name,
							ron,
						} => Ok((root, type_name, ron)),
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "Component",
								received: format!("{:?}", other),
							},
						),
					}?;
				// very confusing to callback to the map like this
				let root = root.bpipe(template_map)?;
				RsxComponent {
					tag,
					tracker,
					type_name,
					ron,
					node: Box::new(root),
					slot_children: Box::new(
						slot_children.into_rsx_node(template_map, rusty_map)?,
					),
					template_directives: template_directives.clone(),
					location,
				}
				.into()
			}
			RsxTemplateNode::RustBlock { tracker, location } => {
				let (initial, effect) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("RustBlock"),
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::RustBlock { initial, effect } => {
							Ok((initial, effect))
						}
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "BlockNode",
								received: format!("{:?}", other),
							},
						),
					}?;
				RsxBlock {
					initial: Box::new(initial),
					effect,
					location,
				}
				.into()
			}
			RsxTemplateNode::Element {
				tag,
				self_closing,
				attributes,
				children,
				location,
			} => RsxElement {
				tag,
				self_closing,
				attributes: attributes
					.into_iter()
					.map(|attr| attr.into_rsx_node(rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(
					children.into_rsx_node(template_map, rusty_map)?,
				),
				location,
			}
			.into(),
		};
		Ok(node)
	}

	/// A simple dfs visitor for an rsx template node
	pub fn visit(&self, mut func: impl FnMut(&Self)) {
		self.visit_inner(&mut func);
	}
	fn visit_inner(&self, func: &mut impl FnMut(&Self)) {
		func(self);
		match self {
			RsxTemplateNode::Fragment { items, .. } => {
				for item in items {
					item.visit_inner(func);
				}
			}
			RsxTemplateNode::Component { slot_children, .. } => {
				slot_children.visit_inner(func);
			}
			RsxTemplateNode::Element { children, .. } => {
				children.visit_inner(func);
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
				let (initial, effect) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							"AttributeBlock",
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::AttributeBlock {
							initial,
							effect: register,
						} => Ok((initial, register)),
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "AttributeBlock",
								received: format!("{:?}", other),
							},
						),
					}?;

				Ok(RsxAttribute::Block { initial, effect })
			}
			RsxTemplateAttribute::BlockValue { key, tracker } => {
				let (initial, effect) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							"AttributeValue",
							rusty_map,
							tracker,
						)
					})? {
						RustyPart::AttributeValue {
							initial,
							effect: register,
						} => Ok((initial, register)),
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
					effect,
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
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		let tracker = RustyTracker::new(0, 15046980652419922415);
		let node = rsx_template! { <div>{value}</div> };

		expect(&node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			location: None,
			children: Box::new(RsxTemplateNode::RustBlock {
				tracker,
				location: None,
			}),
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

		expect(&template).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			location: None,
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
				location: None,
				children: Box::new(RsxTemplateNode::Fragment {
					location: None,
					items: vec![
						RsxTemplateNode::Text {
							location: None,
							value: "\n\t\t\t\t\thello ".to_string(),
						},
						RsxTemplateNode::Component {
							location: None,
							tracker: component_tracker,
							tag: "MyComponent".to_string(),
							slot_children: Box::new(RsxTemplateNode::Element {
								tag: "div".to_string(),
								self_closing: false,
								attributes: vec![],
								location: None,
								children: Box::new(RsxTemplateNode::Text {
									value: "some child".to_string(),
									location: None,
								}),
							}),
							template_directives: vec![],
						},
					],
				}),
			}),
		});
	}

	#[test]
	fn ron() {
		// whats this testing? its already ron
		let template = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let template2 = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		expect(template).to_be(template2);
	}
}
