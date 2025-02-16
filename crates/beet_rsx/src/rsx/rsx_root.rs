use crate::prelude::*;
use anyhow::Result;
use std::borrow::Borrow;
use std::borrow::BorrowMut;

/// This is an RsxNode and a location, which is required for hydration.
///
///
/// The struct returned from an rsx! macro.
#[derive(Debug, Default)]
pub struct RsxRoot {
	/// the root node
	pub node: RsxNode,
	/// unique location with file, line, col
	pub location: RsxLocation,
}

impl RsxRoot {
	/// This is the method used by routers,
	/// applies styles and slots, returning an HtmlDocument.
	pub fn build_document(mut self) -> Result<HtmlDocument> {
		ScopedStyle::default().apply(&mut self)?;
		SlotsVisitor::apply(&mut self)?;
		let html = RsxToHtml::default().map_node(&self);
		let doc = html.into_document();
		Ok(doc)
	}
	/// convenience method usually for testing:
	/// - [ScopedStyle::apply]
	/// - [SlotsVisitor::apply]
	/// - [RsxToHtml::map_node]
	/// - [HtmlNode::render]
	///
	/// # Panics
	/// If the slots cannot be applied.
	pub fn render_body(mut self) -> String {
		ScopedStyle::default().apply(&mut self).unwrap();
		SlotsVisitor::apply(&mut self).unwrap();
		let html = RsxToHtml::default().map_node(&self);
		html.render()
	}
}

impl std::ops::Deref for RsxRoot {
	type Target = RsxNode;
	fn deref(&self) -> &Self::Target { &self.node }
}
impl std::ops::DerefMut for RsxRoot {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.node }
}

impl AsRef<RsxNode> for RsxRoot {
	fn as_ref(&self) -> &RsxNode { &self.node }
}
impl AsMut<RsxNode> for RsxRoot {
	fn as_mut(&mut self) -> &mut RsxNode { &mut self.node }
}

impl Borrow<RsxNode> for RsxRoot {
	fn borrow(&self) -> &RsxNode { &self.node }
}
impl BorrowMut<RsxNode> for RsxRoot {
	fn borrow_mut(&mut self) -> &mut RsxNode { &mut self.node }
}
