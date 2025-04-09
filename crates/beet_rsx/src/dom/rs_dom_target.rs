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


pub struct MountRsDom;

impl Pipeline<RsxNode, Result<RsxNode>> for MountRsDom {
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
		rsx: RsxNode,
		loc: TreeLocation,
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
		RsxNode::Doctype(_) => todo!(),
		RsxNode::Comment(_) => todo!(),
		RsxNode::Fragment(_) => todo!(),
		RsxNode::Component(_) => todo!(),
		RsxNode::Block(_) => todo!(),
		RsxNode::Element(rsx_element) => todo!(),
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
