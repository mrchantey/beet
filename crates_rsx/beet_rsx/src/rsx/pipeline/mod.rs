#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod apply_fs_src;
mod apply_slots;
mod build_step;
mod client_island;
mod register_effects;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use apply_fs_src::*;
pub use apply_slots::*;
pub use build_step::*;
pub use client_island::*;
pub use register_effects::*;
#[cfg(feature = "css")]
mod apply_scoped_style;
#[cfg(feature = "css")]
pub use apply_scoped_style::*;

use crate::prelude::*;
use anyhow::Result;

#[derive(Default)]
pub struct DefaultRsxTransforms {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	fs_src: ApplyFsSrc,
	#[cfg(feature = "css")]
	scoped_style: ApplyScopedStyle,
	slots: ApplySlots,
}

impl Pipeline<RsxNode, Result<RsxNode>> for DefaultRsxTransforms {
	fn apply(self, root: RsxNode) -> Result<RsxNode> {
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		let root = root.xpipe(self.fs_src)?;
		#[cfg(feature = "css")]
		let root = root.xpipe(self.scoped_style)?;
		let root = root.xpipe(self.slots)?;
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

impl Pipeline<RsxNode, Result<HtmlDocument>> for RsxToHtmlDocument {
	fn apply(self, mut node: RsxNode) -> Result<HtmlDocument> {
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
}

impl Pipeline<RsxNode, Result<String>> for RsxToHtmlString {
	fn apply(self, root: RsxNode) -> Result<String> {
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
	fn my_component(_: MyComponent) -> RsxNode {
		rsx! { <div /> }
	}

	#[test]
	fn auto_resumable() {
		let doc = rsx! { <MyComponent /> }
			.xpipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(1);
		let doc = rsx! { <MyComponent client:load /> }
			.xpipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(4);
	}
}
