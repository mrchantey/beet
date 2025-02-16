use crate::prelude::*;

/// An implementation of hydrator that simply updates a tree of
/// html nodes.
pub struct HtmlNodeHydrator {
	pub html: HtmlDocument,
	constants: HtmlConstants,
	loc_map: DomLocationMap,
}

impl HtmlNodeHydrator {
	pub fn new(rsx: impl Rsx) -> Self {
		let rsx = rsx.into_rsx();
		let html = RsxToResumableHtml::default().map_node(&rsx);

		let loc_map = DomLocationMap::from_node(&rsx);

		Self {
			html,
			constants: Default::default(),
			loc_map,
		}
	}
}

impl DomHydrator for HtmlNodeHydrator {
	fn html_constants(&self) -> &HtmlConstants { &self.constants }

	fn render(&self) -> String { self.html.render() }

	fn update_rsx_node(
		&mut self,
		rsx: RsxNode,
		loc: DomLocation,
	) -> ParseResult<()> {
		let parent_idx = self
			.loc_map
			.rusty_locations
			.get(&loc.rsx_idx)
			.ok_or_else(|| {
				ParseError::Hydration(format!(
					"Could not find block parent for index: {}",
					loc.rsx_idx
				))
			})?
			.parent_idx
			.to_string();

		for html in self.html.iter_mut() {
			if let Some(parent_el) = html.query_selector_attr(
				self.constants.rsx_idx_key,
				Some(&parent_idx),
			) {
				return apply_rsx(parent_el, rsx, loc, &self.constants);
			}
		}

		return Err(ParseError::Hydration(format!(
			"Could not find node with id: {}",
			parent_idx
		)));
	}
}


/// we've found a html node with a matching id
#[allow(unused)]
fn apply_rsx(
	parent_el: &mut HtmlElementNode,
	rsx: RsxNode,
	loc: DomLocation,
	constants: &HtmlConstants,
) -> ParseResult<()> {
	match rsx {
		RsxNode::Fragment(vec) => todo!(),
		RsxNode::Component(_) => todo!(),
		RsxNode::Block(RsxBlock { initial, effect }) => todo!(),
		RsxNode::Element(rsx_element) => todo!(),
		RsxNode::Text(text) => {
			let child =
				parent_el.children.get_mut(loc.child_idx as usize).ok_or_else(|| {
					ParseError::Hydration(format!(
						"child node at index: {} is out of bounds. Maybe the text nodes weren't expanded",
						loc.child_idx,
					))
				})?;
			*child = HtmlNode::Text(text);
		}
		RsxNode::Comment(_) => todo!(),
		RsxNode::Doctype => todo!(),
	}
	Ok(())
}
