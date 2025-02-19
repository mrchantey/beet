use crate::prelude::*;

/// An implementation of hydrator that simply updates a tree of
/// html nodes.
pub struct RsDomTarget {
	pub html: HtmlDocument,
	constants: HtmlConstants,
	loc_map: TreeLocationMap,
}

impl RsDomTarget {
	pub fn new(root: &RsxRoot) -> Self {
		let html = RsxToResumableHtml::default().map_root(root);
		let loc_map = TreeLocationMap::from_node(root);
		Self {
			html,
			constants: Default::default(),
			loc_map,
		}
	}
}

impl DomTargetImpl for RsDomTarget {
	fn html_constants(&self) -> &HtmlConstants { &self.constants }

	fn render(&self) -> String { self.html.render() }

	fn update_rsx_node(
		&mut self,
		rsx: RsxNode,
		loc: TreeLocation,
	) -> ParseResult<()> {
		let parent_idx = self
			.loc_map
			.rusty_locations
			.get(&loc.tree_idx)
			.ok_or_else(|| {
				ParseError::Hydration(format!(
					"Could not find block parent for tree index: {}",
					loc.tree_idx
				))
			})?
			.parent_idx
			.to_string();

		for html in self.html.iter_mut() {
			// let parent_hash =
			if let Some(parent_el) = html.query_selector_attr(
				self.constants.tree_idx_key,
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
	loc: TreeLocation,
	constants: &HtmlConstants,
) -> ParseResult<()> {
	match rsx {
		RsxNode::Fragment { .. } => todo!(),
		RsxNode::Component(_) => todo!(),
		RsxNode::Block(RsxBlock { .. }) => todo!(),
		RsxNode::Element(rsx_element) => todo!(),
		RsxNode::Text { value, .. } => {
			let child =
				parent_el.children.get_mut(loc.child_idx as usize).ok_or_else(|| {
					ParseError::Hydration(format!(
						"child node at index: {} is out of bounds. Maybe the text nodes weren't expanded",
						loc.child_idx,
					))
				})?;
			*child = HtmlNode::Text(value);
		}
		RsxNode::Comment { .. } => todo!(),
		RsxNode::Doctype { .. } => todo!(),
	}
	Ok(())
}
