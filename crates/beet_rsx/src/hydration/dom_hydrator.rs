use super::rsx_context_map::RsxContextMap;
use crate::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::Document;
use web_sys::Element;
use web_sys::Text;

/// A hydrator for working with the dom
pub struct DomHydrator {
	constants: HtmlConstants,
	// cache document reference
	document: Document,
	/// sparse set element array, cached for fast reference
	elements: Vec<Option<Element>>,
	cx_map: Option<RsxContextMap>,
}

impl Default for DomHydrator {
	fn default() -> Self {
		Self {
			constants: Default::default(),
			document: window().unwrap().document().unwrap(),
			elements: Default::default(),
			cx_map: Default::default(),
		}
	}
}

impl DomHydrator {
	fn get_cx_map(&mut self) -> ParseResult<&RsxContextMap> {
		let query = format!("[{}]", self.constants.cx_map_key,);
		if let Some(cx) = self.document.query_selector(&query).unwrap() {
			let inner_text = cx.text_content().unwrap();
			self.cx_map = Some(RsxContextMap::from_csv(&inner_text)?);
			Ok(&self.cx_map.as_ref().unwrap())
		} else {
			Err(ParseError::serde(format!(
				"Could not find context attribute: {}",
				query
			)))
		}
	}

	/// we've found a html node with a matching id
	#[allow(unused)]
	fn apply_rsx(
		&self,
		el: Element,
		rsx: RsxNode,
		cx: &RsxContext,
	) -> ParseResult<()> {
		Ok(())
	}

	/// try to get cached element or find it in the dom.
	/// This also uncollapses the child text nodes
	fn get_or_find_element(&mut self, cx: &RsxContext) -> ParseResult<Element> {
		if let Some(Some(el)) = self.elements.get(cx.element_idx()) {
			return Ok(el.clone());
		}
		let id = cx.element_idx();

		let query = format!("[{}='{}']", self.constants.id_key, id);
		if let Some(el) = self.document.query_selector(&query).unwrap() {
			self.elements.resize(id + 1, None);
			self.elements[id] = Some(el.clone());
			self.uncollapse_child_text_nodes(&el, id)?;
			Ok(el)
		} else {
			Err(ParseError::Hydration(format!(
				"Could not find collapsed text node parent with id: {}",
				id
			)))
		}
	}

	/// use the cx_map to uncollapse text nodes
	fn uncollapse_child_text_nodes(
		&mut self,
		el: &Element,
		rsx_id: usize,
	) -> ParseResult<()> {
		let children = el.child_nodes();
		let cx_map = self.get_cx_map()?;
		let Some(el_cx) = cx_map.collapsed_elements.get(&rsx_id) else {
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


impl Hydrator for DomHydrator {
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
		rsx: RsxNode,
		cx: &RsxContext,
	) -> ParseResult<()> {
		let el = self.get_or_find_element(cx)?;
		let child =
			el.child_nodes()
				.item(cx.child_idx() as u32)
				.ok_or_else(|| {
					ParseError::Hydration("Could not find child".into())
				})?;

		#[allow(unused)]
		match rsx {
			RsxNode::Block {
				initial,
				register_effect,
			} => {
				todo!()
			}
			RsxNode::Text(val) => {
				if let Some(child) = child.dyn_ref::<Text>() {
					child.set_text_content(Some(&val));
				} else {
					todo!("replace with text node");
				}
			}
			RsxNode::Fragment(vec) => todo!(),
			RsxNode::Doctype => todo!(),
			RsxNode::Comment(_) => todo!(),
			RsxNode::Element(rsx_element) => todo!(),
		}


		Ok(())
	}
}
