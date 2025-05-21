mod remove_lang_templates;
pub use remove_lang_templates::*;
mod apply_slots;
mod build_step;
mod client_island;
mod register_effects;
use crate::prelude::*;
use anyhow::Result;
pub use apply_slots::*;
use beet_common::prelude::*;
pub use build_step::*;
pub use client_island::*;
pub use register_effects::*;


#[derive(Default)]
pub struct DefaultRsxTransforms {
	slots: ApplySlots,
	// remove_lang_templates: RemoveLangTemplates,
}
impl Pipeline<WebNode, Result<WebNode>> for DefaultRsxTransforms {
	fn apply(self, root: WebNode) -> Result<WebNode> {
		let root = root.xpipe(self.slots)?;
        // .xpipe(self.remove_lang_templates);
		Ok(root)
	}
}
#[derive(Default)]
pub struct RsxToHtmlDocument {
	pub rsx_transforms: DefaultRsxTransforms,
	pub rsx_to_html: RsxToHtml,
	pub html_to_document: HtmlToDocument,
	pub html_doc_to_resumable: HtmlDocToResumable,
}
impl Pipeline<WebNode, Result<HtmlDocument>> for RsxToHtmlDocument {
	fn apply(self, mut node: WebNode) -> Result<HtmlDocument> {
		node = node.xpipe(self.rsx_transforms)?;
		let mut client_reactive = false;
		VisitRsxComponent::new(|c| {
			if c.is_client_reactive() {
				client_reactive = true;
			}
		})
		.walk_node(&node);
		let mut doc = node
			.xref()
			.xpipe(self.rsx_to_html)
			.xpipe(self.html_to_document)?;
		if client_reactive {
			doc = doc
				.xmap(|doc| (doc, node))
				.xpipe(self.html_doc_to_resumable);
		}
		Ok(doc)
	}
}
/// used for testing, directly transform rsx root then parse to html string
#[derive(Default)]
pub struct RsxToHtmlString {
	pub rsx_transforms: DefaultRsxTransforms,
	pub rsx_to_html: RsxToHtml,
	pub render_html: RenderHtmlEscaped,
}
impl RsxToHtmlString {
	pub fn no_slot_check(mut self) -> Self {
		self.rsx_to_html.no_slot_check = true;
		self
	}
	pub fn trim(mut self) -> Self {
		self.rsx_to_html.trim = true;
		self
	}
}
impl Pipeline<WebNode, Result<String>> for RsxToHtmlString {
	fn apply(self, root: WebNode) -> Result<String> {
		root.xpipe(self.rsx_transforms)?
			.xref()
			.xpipe(self.rsx_to_html)
			.xpipe(self.render_html)
			.xok()
	}
}
#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;
	#[derive(Node, Serialize, Deserialize)]
	struct MyComponent;
	fn my_component(_: MyComponent) -> WebNode {
		rsx! {
			< div />
		}
	}
	#[test]
	fn auto_resumable() {
		let doc = rsx! {
			< MyComponent />
		}
		.xpipe(RsxToHtmlDocument::default())
		.unwrap();
		expect(doc.body.len()).to_be(1);
		let doc = rsx! {
			< MyComponent client : load />
		}
		.xpipe(RsxToHtmlDocument::default())
		.unwrap();
		expect(doc.body.len()).to_be(4);
	}
}
