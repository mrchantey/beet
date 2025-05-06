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
	Doctype { meta: RsxNodeMeta },
	/// Serializable [`RsxNode::Comment`]
	Comment { value: String, meta: RsxNodeMeta },
	/// Serializable [`RsxNode::Text`]
	Text { value: String, meta: RsxNodeMeta },
	/// Serializable [`RsxNode::Fragment`]
	Fragment { items: Vec<Self>, meta: RsxNodeMeta },
	/// Serializable [`RsxNode::Block`]
	/// the initial value is the responsibility of the [RustyPart::RustBlock]
	RustBlock {
		tracker: RustyTracker,
		meta: RsxNodeMeta,
	},
	/// Serializable [`RsxNode::Element`]
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Box<Self>,
		meta: RsxNodeMeta,
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
		meta: RsxNodeMeta,
	},
}

pub type TemplateResult<T> = std::result::Result<T, TemplateError>;

impl Default for RsxTemplateNode {
	fn default() -> Self {
		Self::Fragment {
			items: Default::default(),
			meta: Default::default(),
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
	#[error(r#"
`RustyPartMap` is missing a tracker for {cx}
Expected: {expected:#?}
Received: {received:#?}

---

Congratulations you've reached a hydration error! 🚀

Its likely that you can see a corresponding index but a different hash in the error.
This may be caused by one of several reasons:
	1. The generated templates are out of sync with the compiled rust code.
	2. A new discrepancy between how the rsx! macro creates a TokenStream and parsing
		a syn::File does it. For example they handle whitespace differently.

Please try a full rebuild and file a reproducible issue if that doesn't work.

## Debugging (for contributors)

The two entrypoints for the tracker generation are
			- `BuildTemplateMap` which creates an `RsxTemplateMap` via the `RsxRonPipeline`
			- The `rsx!` macro which creates rusty trackers via the `RsxMacroPipeline`
			
A good place to start with println! are in the RustyTrackerBuilder which
handles *all* hash generation for both macros and file loading.
have a good look at the tokens being passed in and check they match

Also remember that using the rsx_template! macro likely wont help because that uses the 
same process as the rsx! macro. It would be better to use syn::parse or something closer to
the syn::parse_file 
---

"#
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
		received_map: &RustyPartMap,
		expected: RustyTracker,
	) -> Self {
		Self::NoRustyMap {
			cx: cx.to_string(),
			received: received_map.keys().cloned().collect(),
			expected,
		}
	}
}

impl NodeMeta for RsxTemplateNode {
	fn meta(&self) -> &RsxNodeMeta {
		match self {
			RsxTemplateNode::Doctype { meta }
			| RsxTemplateNode::Comment { meta, .. }
			| RsxTemplateNode::Text { meta, .. }
			| RsxTemplateNode::Fragment { meta, .. }
			| RsxTemplateNode::RustBlock { meta, .. }
			| RsxTemplateNode::Element { meta, .. }
			| RsxTemplateNode::Component { meta, .. } => meta,
		}
	}

	fn meta_mut(&mut self) -> &mut RsxNodeMeta {
		match self {
			RsxTemplateNode::Doctype { meta }
			| RsxTemplateNode::Comment { meta, .. }
			| RsxTemplateNode::Text { meta, .. }
			| RsxTemplateNode::Fragment { meta, .. }
			| RsxTemplateNode::RustBlock { meta, .. }
			| RsxTemplateNode::Element { meta, .. }
			| RsxTemplateNode::Component { meta, .. } => meta,
		}
	}
}

impl RsxTemplateNode {
	#[cfg(feature = "serde")]
	pub fn from_ron(ron: &str) -> anyhow::Result<Self> {
		ron::de::from_str(ron).map_err(Into::into)
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
		let tracker = RustyTracker::new(0, 14909846839018434065);
		// Element (tag : \"div\" , self_closing : true , attributes : [] , children : Fragment (items : [] , meta : RsxNodeMeta (template_directives : [] , location : None)) , meta : RsxNodeMeta (template_directives : [] , location : None))
		let node = rsx_template! { <div>{value}</div> };

		expect(&node).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			meta: RsxNodeMeta::default(),
			children: Box::new(RsxTemplateNode::RustBlock {
				tracker,
				meta: RsxNodeMeta::default(),
			}),
		});
	}
	#[test]
	fn complex() {
		let ident_tracker = RustyTracker::new(0, 6068255516074130633);
		let component_tracker = RustyTracker::new(1, 4498377743695909661);
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
			meta: RsxNodeMeta::default(),

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
				meta: RsxNodeMeta::default(),

				children: Box::new(RsxTemplateNode::Fragment {
					meta: RsxNodeMeta::default(),

					items: vec![
						RsxTemplateNode::Text {
							meta: RsxNodeMeta::default(),

							value: "\n\t\t\t\t\thello ".to_string(),
						},
						RsxTemplateNode::Component {
							meta: RsxNodeMeta::default(),

							tracker: component_tracker,
							tag: "MyComponent".to_string(),
							slot_children: Box::new(RsxTemplateNode::Element {
								tag: "div".to_string(),
								self_closing: false,
								attributes: vec![],
								meta: RsxNodeMeta::default(),

								children: Box::new(RsxTemplateNode::Text {
									value: "some child".to_string(),
									meta: RsxNodeMeta::default(),
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
