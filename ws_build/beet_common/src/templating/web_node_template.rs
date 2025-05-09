use crate::prelude::*;
use thiserror::Error;


/// Serializable version of an web node that can be rehydrated.
///
/// An [WebNodeTemplate] is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
///
/// Templates do not recurse into rusty parts,
/// ie [`RsxBlock::initial`] or [`RsxComponent::node`] are not recursed into.
/// For this reason its important that the [`RsxTemplateMap`] visits these
/// children when applying the templates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebNodeTemplate {
	/// Serializable [`WebNode::Doctype`]
	Doctype { meta: NodeMeta },
	/// Serializable [`WebNode::Comment`]
	Comment { value: String, meta: NodeMeta },
	/// Serializable [`WebNode::Text`]
	Text { value: String, meta: NodeMeta },
	/// Serializable [`WebNode::Fragment`]
	Fragment { items: Vec<Self>, meta: NodeMeta },
	/// Serializable [`WebNode::Block`]
	/// the initial value is the responsibility of the [RustyPart::RustBlock]
	RustBlock {
		tracker: RustyTracker,
		meta: NodeMeta,
	},
	/// Serializable [`WebNode::Element`]
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Box<Self>,
		meta: NodeMeta,
	},
	/// Serializable [`WebNode::Component`]
	/// We dont know much about components, for example when parsing
	/// a file we just get the name.
	/// The [FileSpan] etc is is tracked by the [RustyPart::Component::root]
	Component {
		/// the hydrated part has the juicy details
		tracker: RustyTracker,
		tag: String,
		/// mapped from [RsxComponent::slot_children]
		slot_children: Box<Self>,
		meta: NodeMeta,
	},
}


pub type TemplateResult<T> = std::result::Result<T, TemplateError>;

impl Default for WebNodeTemplate {
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
		"WebNode has no tracker for {0}, ensure they are included in RstmlToRsx settings"
	)]
	DehydrationFailed(String),
	#[error(
		"No template found\nExpected: {expected:#?}\nReceived: {received:#?}"
	)]
	NoTemplate {
		expected: FileSpan,
		received: Vec<FileSpan>,
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
			- `BuildTemplateMap` which creates an `RsxTemplateMap` via `FileToTemplates`
			- The `rsx!` macro which creates rusty trackers via `RsxMacroPipeline`
			
A good place to start with println! are in the RustyTrackerBuilder which
handles *all* hash generation for both macros and file loading.
have a good look at the tokens being passed in and check they match

Also remember that using the rsx_template! macro likely wont help because that uses the 
same process as the rsx! macro. It would be better to use syn::parse or something closer to
the syn::parse_file workflow.
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
	#[error("Location: {location}\nError: {err}")]
	WithLocation { location: FileSpan, err: Box<Self> },
}

impl TemplateError {
	pub fn with_location(self, location: FileSpan) -> Self {
		Self::WithLocation {
			location,
			err: Box::new(self),
		}
	}

	pub fn no_rusty_map<'a>(
		cx: &str,
		received: impl IntoIterator<Item = &'a RustyTracker>,
		expected: RustyTracker,
	) -> Self {
		Self::NoRustyMap {
			cx: cx.to_string(),
			received: received
				.into_iter()
				.map(|tracker| (*tracker).clone())
				.collect(),
			expected,
		}
	}
}

impl GetNodeMeta for WebNodeTemplate {
	fn meta(&self) -> &NodeMeta {
		match self {
			WebNodeTemplate::Doctype { meta }
			| WebNodeTemplate::Comment { meta, .. }
			| WebNodeTemplate::Text { meta, .. }
			| WebNodeTemplate::Fragment { meta, .. }
			| WebNodeTemplate::RustBlock { meta, .. }
			| WebNodeTemplate::Element { meta, .. }
			| WebNodeTemplate::Component { meta, .. } => meta,
		}
	}

	fn meta_mut(&mut self) -> &mut NodeMeta {
		match self {
			WebNodeTemplate::Doctype { meta }
			| WebNodeTemplate::Comment { meta, .. }
			| WebNodeTemplate::Text { meta, .. }
			| WebNodeTemplate::Fragment { meta, .. }
			| WebNodeTemplate::RustBlock { meta, .. }
			| WebNodeTemplate::Element { meta, .. }
			| WebNodeTemplate::Component { meta, .. } => meta,
		}
	}
}

impl WebNodeTemplate {
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
			WebNodeTemplate::Fragment { items, .. } => {
				for item in items {
					item.visit_inner(func);
				}
			}
			WebNodeTemplate::Component { slot_children, .. } => {
				slot_children.visit_inner(func);
			}
			WebNodeTemplate::Element { children, .. } => {
				children.visit_inner(func);
			}
			_ => {}
		}
	}
	pub fn visit_mut(&mut self, mut func: impl FnMut(&mut Self)) {
		self.visit_inner_mut(&mut func);
	}
	fn visit_inner_mut(&mut self, func: &mut impl FnMut(&mut Self)) {
		func(self);
		match self {
			WebNodeTemplate::Fragment { items, .. } => {
				for item in items.iter_mut() {
					item.visit_inner_mut(func);
				}
			}
			WebNodeTemplate::Component { slot_children, .. } => {
				slot_children.visit_inner_mut(func);
			}
			WebNodeTemplate::Element { children, .. } => {
				children.visit_inner_mut(func);
			}
			_ => {}
		}
	}
	/// When testing for equality sometimes we dont want to compare spans and trackers.
	pub fn reset_spans_and_trackers(mut self) -> Self {
		self.visit_mut(|node| {
			*node.meta_mut().span_mut() = FileSpan::default();
			match node {
				WebNodeTemplate::RustBlock { tracker, .. } => {
					*tracker = RustyTracker::PLACEHOLDER;
				}
				WebNodeTemplate::Component { tracker, .. } => {
					*tracker = RustyTracker::PLACEHOLDER;
				}
				WebNodeTemplate::Element { attributes, .. } => {
					attributes.iter_mut().for_each(|attr| match attr {
						RsxTemplateAttribute::Block(tracker) => {
							*tracker = RustyTracker::PLACEHOLDER
						}
						RsxTemplateAttribute::BlockValue {
							tracker, ..
						} => *tracker = RustyTracker::PLACEHOLDER,
						_ => {}
					})
				}
				_ => {}
			}
		});
		self
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

#[cfg(feature = "tokens")]
use std::str::FromStr;

#[cfg(feature = "tokens")]
impl RustTokens for WebNodeTemplate {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let ron = ron::ser::to_string(self).unwrap();
		proc_macro2::TokenStream::from_str(&ron).unwrap()
	}
}
