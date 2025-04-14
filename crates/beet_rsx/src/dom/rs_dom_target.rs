use crate::prelude::*;
use anyhow::Result;
/// An implementation of hydrator that simply updates a tree of
/// html nodes. This is conceptually similar to JSDom in that
/// it mocks the dom.
pub struct RsDomTarget {
	pub doc: HtmlDocument,
	constants: HtmlConstants,
	loc_map: TreeLocationMap,
}


pub struct MountToRsDom;

impl Pipeline<RsxNode, Result<RsxNode>> for MountToRsDom {
	fn apply(self, root: RsxNode) -> Result<RsxNode> {
		DomTarget::set(RsDomTarget::new(&root)?);
		Ok(root)
	}
}

impl RsDomTarget {
	/// This does *not* apply any transformations
	pub fn new(root: &RsxNode) -> Result<Self> {
		let doc = root
			.xpipe(RsxToHtml::default())
			.xpipe(HtmlToDocument::default())?;

		let loc_map = root.xpipe(NodeToTreeLocationMap);
		Ok(Self {
			doc,
			loc_map,
			constants: Default::default(),
		})
	}
}

impl DomTargetImpl for RsDomTarget {
	fn tree_location_map(&mut self) -> &TreeLocationMap { &self.loc_map }
	fn html_constants(&self) -> &HtmlConstants { &self.constants }

	fn render(&self) -> String {
		self.doc.clone().xpipe(RenderHtmlEscaped::default())
	}

	fn update_rsx_node(
		&mut self,
		loc: TreeLocation,
		rsx: RsxNode,
	) -> ParseResult<()> {
		for html in self.doc.iter_mut() {
			if let Some(parent_el) = html.query_selector_attr(
				self.constants.tree_idx_key,
				Some(&loc.parent_idx.to_string()),
			) {
				return apply_rsx(parent_el, rsx, loc, &self.constants);
			}
		}
		return Err(ParseError::Hydration(format!(
			"Could not find node with id: {}",
			loc.parent_idx
		)));
	}

	fn update_rsx_attribute(
		&mut self,
		loc: TreeLocation,
		key: &str,
		value: &str,
	) -> ParseResult<()> {
		for html in self.doc.iter_mut() {
			if let Some(el) = html.query_selector_attr(
				self.constants.tree_idx_key,
				Some(&loc.tree_idx.to_string()),
			) {
				el.set_attribute(key, value);
				return Ok(());
			}
		}
		return Err(ParseError::Hydration(format!(
			"Could not find node with id: {}",
			loc.tree_idx
		)));
	}
}


/// we've found a html node with a matching id
fn apply_rsx(
	parent_el: &mut HtmlElementNode,
	rsx: RsxNode,
	loc: TreeLocation,
	_constants: &HtmlConstants,
) -> ParseResult<()> {
	match rsx {
		RsxNode::Doctype(_) => todo!(),
		RsxNode::Comment(_) => todo!(),
		RsxNode::Fragment(_) => todo!(),
		RsxNode::Component(_) => todo!(),
		RsxNode::Block(_) => todo!(),
		RsxNode::Element(_) => todo!(),
		RsxNode::Text(text) => {
			let child =
				parent_el.children.get_mut(loc.child_idx as usize).ok_or_else(|| {
					ParseError::Hydration(format!(
						"child node at index: {} is out of bounds. Maybe the text nodes weren't expanded",
						loc.child_idx,
					))
				})?;
			*child = HtmlNode::Text(text.value);
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		rsx! { <div data-beet-rsx-idx="0">value</div> }
			.xpipe(MountToRsDom)
			.unwrap();

		// a text node will use the parent idx
		DomTarget::update_rsx_node(TreeLocation::new(1, 0, 0), rsx! {bar})
			.unwrap();
		expect(DomTarget::render()).to_be("<!DOCTYPE html><html><head></head><body><div data-beet-rsx-idx=\"0\">bar</div></body></html>");

		DomTarget::update_rsx_attribute(
			TreeLocation::new(0, 0, 0),
			"wheres-the",
			"any-key",
		)
		.unwrap();
		expect(DomTarget::render()).to_be("<!DOCTYPE html><html><head></head><body><div data-beet-rsx-idx=\"0\" wheres-the=\"any-key\">bar</div></body></html>");
	}
}
