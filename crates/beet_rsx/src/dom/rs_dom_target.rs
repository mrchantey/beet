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

impl RsxPipeline<RsxRoot, Result<RsxRoot>> for MountRsDom {
	fn apply(self, root: RsxRoot) -> Result<RsxRoot> {
		DomTarget::set(RsDomTarget::new(&root)?);
		Ok(root)
	}
}

impl RsDomTarget {
	/// This does *not* apply any transformations
	pub fn new(root: &RsxRoot) -> Result<Self> {
		let doc = root
			.bpipe(RsxToHtml::default())
			.bpipe(HtmlToDocument::default())?;

		let loc_map = root.bpipe(NodeToTreeLocationMap);
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
		self.doc.clone().bpipe(RenderHtml::default()).unwrap()
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
