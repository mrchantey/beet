/// Unlike RsxNode, this struct contains only real html nodes
#[derive(Debug, Clone)]
pub enum HtmlNode {
	Doctype,
	Comment(String),
	Text(String),
	Element(HtmlElementNode),
}

impl HtmlNode {
	/// recursively search for an html node with a matching id
	pub fn query_selector_attr(
		&mut self,
		key: &str,
		val: Option<&str>,
	) -> Option<&mut HtmlElementNode> {
		match self {
			HtmlNode::Element(e) => {
				if e.query_selector_attr(key, val) {
					return Some(e);
				}
				for child in &mut e.children {
					if let Some(node) = child.query_selector_attr(key, val) {
						return Some(node);
					}
				}
			}
			_ => {}
		}
		None
	}

	/// return self as an element if it matches the tag
	pub fn element_with_tag_owned(
		self,
		tag: &str,
	) -> Result<HtmlElementNode, Self> {
		match self {
			HtmlNode::Element(e) => {
				if e.tag == tag {
					return Ok(e);
				}
				Err(e.into())
			}
			_ => Err(self),
		}
	}
	/// return self as an element if it matches the tag
	pub fn element_with_tag(&self, tag: &str) -> Option<&HtmlElementNode> {
		match self {
			HtmlNode::Element(e) => {
				if e.tag == tag {
					return Some(e);
				}
			}
			_ => {}
		}
		None
	}
}
#[derive(Debug, Clone)]
pub struct HtmlElementNode {
	pub tag: String,
	pub self_closing: bool,
	pub attributes: Vec<HtmlAttribute>,
	pub children: Vec<HtmlNode>,
}



impl Into<HtmlNode> for HtmlElementNode {
	fn into(self) -> HtmlNode { HtmlNode::Element(self) }
}

impl HtmlElementNode {
	pub fn assert_self_closing_no_children(&self) {
		if self.self_closing && !self.children.is_empty() {
			panic!("Self closing elements cannot have children");
		}
	}

	pub fn inline_script(
		script: String,
		attributes: Vec<HtmlAttribute>,
	) -> Self {
		Self {
			tag: "script".to_string(),
			self_closing: false,
			attributes,
			children: vec![HtmlNode::Text(script)],
		}
	}


	/// returns true if any attribute matches the key and value
	pub fn query_selector_attr(
		&mut self,
		key: &str,
		val: Option<&str>,
	) -> bool {
		self.attributes
			.iter()
			.any(|a| a.key == key && a.value.as_deref() == val)
	}

	/// returns none if the attribute is not found or it has no value
	pub fn get_attribute_value(&self, key: &str) -> Option<&str> {
		for attr in &self.attributes {
			if attr.key == key {
				return attr.value.as_deref();
			}
		}
		None
	}

	/// Sets an attribute, updating it if it already exists,
	/// otherwise adding it to the list.
	pub fn set_attribute(&mut self, key: &str, value: &str) {
		for attr in &mut self.attributes {
			if attr.key == key {
				attr.value = Some(value.to_string());
				return;
			}
		}
		self.attributes.push(HtmlAttribute {
			key: key.to_string(),
			value: Some(value.to_string()),
		});
	}
}

#[derive(Debug, Clone)]
pub struct HtmlAttribute {
	pub key: String,
	pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HtmlConstants {
	/// the attribute for element ids, used for encoding the [TreePosition],
	pub tree_idx_key: &'static str,
	/// Attrubute tagging the [`TreeLocationMap`](crate::prelude::TreeLocationMap)
	pub loc_map_key: &'static str,
	/// the global event handler for all events
	pub event_handler: &'static str,
	/// the global vec that stores prehydrated events
	pub event_store: &'static str,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		Self {
			tree_idx_key: "data-beet-rsx-idx",
			loc_map_key: "data-beet-loc-map",
			event_handler: "_beet_event_handler",
			event_store: "_beet_event_store",
		}
	}
}
