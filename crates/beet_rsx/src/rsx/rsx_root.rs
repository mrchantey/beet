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

impl RsxRoot {}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	struct MyComponent {
		value: usize,
	}
	impl Component for MyComponent {
		fn render(self) -> RsxRoot {
			rsx! { <div>the value is {self.value}<slot /></div> }
		}
	}

	// test a roundtrip split/join,
	#[test]
	fn split_join() {
		let some_val = 3;

		let node = || {
			rsx! {
				<div key str="value" num=32 ident=some_val>
					<p>
						hello <MyComponent value=3>
							<div>some child</div>
						</MyComponent>
					</p>
				</div>
			}
		};

		let html1 = node().render_body();
		let template = RsxTemplateRoot::from_rsx(&node()).unwrap();
		let map = RsxTemplateMap::from_template_roots(vec![template]);
		let node2 = map.hydrate(node()).unwrap();
		let html2 = node2.render_body();
		expect(html1).to_be(html2);
	}
	#[test]
	fn rsx_template_match_simple() {
		let some_val = 3;
		let node1 = rsx! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		};
		let node2_template = rsx_template! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		};
		let node1_template = RsxTemplateRoot::from_rsx(&node1).unwrap();
		expect(&node1_template).not().to_be(&node2_template);
		expect(&node1_template.node).to_be(&node2_template.node);
	}
	#[test]
	fn rsx_template_match_complex() {
		let some_val = 3;

		let node1 = rsx! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node1_template = RsxTemplateRoot::from_rsx(&node1).unwrap();
		expect(&node1_template).not().to_be(&node2_template);
		expect(&node1_template.node).to_be(&node2_template.node);
	}
}
