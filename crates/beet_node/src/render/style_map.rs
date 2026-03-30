use alloc::borrow::Cow;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// Maps element names to style values with nesting support.
///
/// Maintains a stack of styles pushed/popped as elements are
/// entered/left, plus a fallback association table for aliasing
/// element names (eg `b` → `strong`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleMap<S> {
	/// Style used when no mapping or association is found.
	default_style: Arc<S>,
	/// Stack of active styles so nested elements restore correctly.
	nesting_stack: Vec<Arc<S>>,
	/// Explicit element-name → style mapping.
	element_map: HashMap<Cow<'static, str>, Arc<S>>,
	/// Fallback mapping: if an element is missing from `element_map`,
	/// look up its association here and use that element's style instead.
	default_associations: HashMap<Cow<'static, str>, Cow<'static, str>>,
}

impl<S> StyleMap<S> {
	pub fn new(
		default_style: S,
		element_map: Vec<(impl Into<Cow<'static, str>>, S)>,
	) -> Self {
		Self {
			default_style: Arc::new(default_style),
			nesting_stack: Vec::new(),
			element_map: element_map
				.into_iter()
				.map(|(k, v)| (k.into(), Arc::new(v)))
				.collect(),
			default_associations: default_associations(),
		}
	}


	/// Override the fallback association mapping.
	pub fn with_default_associations(
		mut self,
		map: HashMap<Cow<'static, str>, Cow<'static, str>>,
	) -> Self {
		self.default_associations = map;
		self
	}

	/// Override the default style used when no mapping is found.
	pub fn with_default_style(mut self, style: S) -> Self {
		self.default_style = Arc::new(style);
		self
	}

	pub fn resolve(&self, element: &str) -> &S {
		if let Some(style) = self.element_map.get(element) {
			style
		} else if let Some(assoc) = self.default_associations.get(element) {
			self.element_map.get(assoc).unwrap_or(&self.default_style)
		} else {
			&self.default_style
		}
	}

	pub fn push(&mut self, name: &str) -> Arc<S> {
		let style = self.resolve_style(name);
		self.nesting_stack.push(Arc::clone(&style));
		style
	}

	pub fn pop(&mut self) -> Option<Arc<S>> { self.nesting_stack.pop() }

	pub fn current(&self) -> Arc<S> {
		self.nesting_stack
			.last()
			.unwrap_or(&self.default_style)
			.clone()
	}

	/// Resolve the style for an element name, walking through
	/// associations if needed.
	fn resolve_style(&self, name: &str) -> Arc<S> {
		let lower = name.to_ascii_lowercase();

		// direct lookup
		if let Some(style) = self.element_map.get(lower.as_str()) {
			return style.clone();
		}

		// association fallback (one level deep to avoid cycles)
		if let Some(assoc) = self.default_associations.get(lower.as_str()) {
			if let Some(style) = self.element_map.get(assoc.as_ref()) {
				return style.clone();
			}
		}

		self.default_style.clone()
	}
}


fn default_associations() -> HashMap<Cow<'static, str>, Cow<'static, str>> {
	vec![
		("b", "strong"),
		("i", "em"),
		("s", "del"),
		("div", "p"),
		("span", "p"),
		("section", "p"),
		("article", "p"),
		("aside", "blockquote"),
		("nav", "p"),
		("header", "p"),
		("footer", "p"),
		("main", "p"),
		("dt", "strong"),
		("dd", "p"),
		("th", "strong"),
		("td", "p"),
		("sup", "em"),
		("sub", "em"),
	]
	.into_iter()
	.map(|(k, v)| (Cow::Borrowed(k), Cow::Borrowed(v)))
	.collect()
}
