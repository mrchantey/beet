use crate::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Document;
use web_sys::Element;
use web_sys::Text;
use web_sys::window;

/// A hydrator for working with the dom
pub struct BrowserDomTarget {
	constants: HtmlConstants,
	// cache document reference
	document: Document,
	/// sparse set element array, cached for fast reference
	/// TODO bench this
	elements: Vec<Option<Element>>,
	/// Will be None until lazy loaded
	loc_map: Option<TreeLocationMap>,
}

impl Default for BrowserDomTarget {
	fn default() -> Self {
		Self {
			constants: Default::default(),
			document: window().unwrap().document().unwrap(),
			elements: Default::default(),
			loc_map: Default::default(),
		}
	}
}

impl BrowserDomTarget {
	fn get_or_load_tree_location_map(
		&mut self,
	) -> ParseResult<&TreeLocationMap> {
		if self.loc_map.is_some() {
			// for borrow checker
			return Ok(self.loc_map.as_ref().unwrap());
		} else {
			let query = format!("[{}]", self.constants.loc_map_key);
			if let Some(cx) = self.document.query_selector(&query).unwrap() {
				let inner_text = cx.text_content().unwrap();
				let loc_map = ron::de::from_str(&inner_text).map_err(|e| {
					ParseError::serde(format!(
						"Could not parse TreeLocationMap: {}",
						e
					))
				})?;
				self.loc_map = Some(loc_map);
				Ok(&self.loc_map.as_ref().unwrap())
			} else {
				Err(ParseError::serde(format!(
					"Could not find context attribute: {}",
					query
				)))
			}
		}
	}

	/// we've found a html node with a matching id
	#[allow(unused)]
	fn apply_rsx(
		&self,
		el: Element,
		rsx: RsxNode,
		loc: TreeLocation,
	) -> ParseResult<()> {
		Ok(())
	}

	/// try to get cached element or find it in the dom.
	/// When it is found it will uncollapse text nodes,
	/// ie expand into the locations referenced by the [TreeLocation]
	fn get_or_find_element(
		&mut self,
		tree_idx: TreeIdx,
	) -> ParseResult<Element> {
		if let Some(Some(el)) = self.elements.get(*tree_idx as usize) {
			return Ok(el.clone());
		}

		let query = format!("[{}='{}']", self.constants.tree_idx_key, tree_idx);
		if let Some(el) = self.document.query_selector(&query).unwrap() {
			self.elements.resize((*tree_idx + 1) as usize, None);
			self.elements[*tree_idx as usize] = Some(el.clone());
			self.uncollapse_child_text_nodes(&el, tree_idx)?;
			Ok(el)
		} else {
			Err(ParseError::Hydration(format!(
				"Could not text node parent with rsx idx: {}",
				tree_idx
			)))
		}
	}

	/// use the [TreeLocationMap] to uncollapse text nodes
	fn uncollapse_child_text_nodes(
		&mut self,
		el: &Element,
		tree_idx: TreeIdx,
	) -> ParseResult<()> {
		let children = el.child_nodes();
		let loc_map = self.get_or_load_tree_location_map()?;
		let Some(el_cx) = loc_map.collapsed_elements.get(&tree_idx) else {
			// here we assume this is because the element has no children
			// so was not tracked
			// elements without rust children are not tracked
			return Ok(());
		};


		for (child_index, positions) in el_cx.split_positions.iter().enumerate()
		{
			let whole_text_node =
				children.item(child_index as u32).ok_or_else(|| {
					ParseError::Hydration(format!(
						"Could not find child at index: {}",
						child_index
					))
				})?;
			let mut current_node: web_sys::Text =
				whole_text_node.dyn_into().map_err(|_| {
					ParseError::Hydration(format!(
						"Could not convert child to text node"
					))
				})?;

			for position in positions {
				current_node =
					current_node.split_text(*position as u32).unwrap();
			}
		}

		Ok(())
	}
}


impl DomTargetImpl for BrowserDomTarget {
	fn tree_location_map(&mut self) -> &TreeLocationMap {
		self.get_or_load_tree_location_map().unwrap()
	}
	fn html_constants(&self) -> &HtmlConstants { &self.constants }

	/// returns body inner html
	fn render(&self) -> String {
		window()
			.unwrap()
			.document()
			.unwrap()
			.body()
			.unwrap()
			.inner_html()
	}

	fn update_rsx_node(
		&mut self,
		loc: TreeLocation,
		rsx: RsxNode,
	) -> ParseResult<()> {
		let parent = self.get_or_find_element(loc.parent_idx)?;
		let child =
			parent.child_nodes().item(loc.child_idx as u32).ok_or_else(
				|| ParseError::Hydration("Could not find child".into()),
			)?;

		#[allow(unused)]
		match rsx {
			RsxNode::Fragment { .. } => todo!(),
			RsxNode::Component(_) => todo!(),
			RsxNode::Block(RsxBlock { .. }) => {
				todo!()
			}
			RsxNode::Element(rsx_element) => todo!(),
			RsxNode::Text(text) => {
				if let Some(child) = child.dyn_ref::<Text>() {
					child.set_text_content(Some(&text.value));
				} else {
					todo!(
						"the structure containing reactivity changed, replace with text node?"
					);
				}
			}
			RsxNode::Comment { .. } => todo!(),
			RsxNode::Doctype { .. } => todo!(),
		}


		Ok(())
	}
}
