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


/// Basically a `FnOnce` trait, but not nightly and a little less awkward to implement.
pub trait RsxPipeline<In, Out = In> {
	/// Consume self and apply to the target
	fn apply(self, value: In) -> Out;
}

impl<F, In, Out> RsxPipeline<In, Out> for F
where
	F: FnOnce(In) -> Out,
{
	fn apply(self, value: In) -> Out { self(value) }
}


/// Utilities for method-chaining on any type.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait RsxPipelineTarget: Sized {
	/// its like map but for any type
	fn bmap<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }
	/// its like inpsect but for any type
	fn btap(mut self, func: impl FnOnce(&mut Self)) -> Self {
		func(&mut self);
		self
	}
	fn btap_mut(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
		func(self);
		self
	}
	/// its like map but for our pipeline trait
	fn bpipe<P: RsxPipeline<Self, O>, O>(self, pipeline: P) -> O {
		pipeline.apply(self)
	}

	fn bref(&self) -> &Self { self }
}
impl<T: Sized> RsxPipelineTarget for T {}


#[derive(Default)]
pub struct DefaultRsxTransforms {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	fs_src: ApplyFsSrc,
	#[cfg(feature = "css")]
	scoped_style: ApplyScopedStyle,
	slots: ApplySlots,
}

impl RsxPipeline<RsxNode, Result<RsxNode>> for DefaultRsxTransforms {
	fn apply(self, root: RsxNode) -> Result<RsxNode> {
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		let root = root.bpipe(self.fs_src)?;
		#[cfg(feature = "css")]
		let root = root.bpipe(self.scoped_style)?;
		let root = root.bpipe(self.slots)?;
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

impl RsxPipeline<RsxNode, Result<HtmlDocument>> for RsxToHtmlDocument {
	fn apply(self, mut node: RsxNode) -> Result<HtmlDocument> {
		node = node.bpipe(self.rsx_transforms)?;

		let mut client_reactive = false;
		VisitRsxComponent::new(|c| {
			if c.is_client_reactive() {
				client_reactive = true;
			}
		})
		.walk_node(&node);


		let mut doc = node
			.bref()
			.bpipe(self.rsx_to_html)
			.bpipe(self.html_to_document)?;
		if client_reactive {
			doc = doc
				.bmap(|doc| (doc, node))
				.bpipe(self.html_doc_to_resumable);
		}
		Ok(doc)
	}
}

/// used for testing, directly transform rsx root then parse to html string
#[derive(Default)]
pub struct RsxToHtmlString {
	pub rsx_transforms: DefaultRsxTransforms,
	pub rsx_to_html: RsxToHtml,
	pub render_html: RenderHtml,
}

impl RsxToHtmlString {
	pub fn no_slot_check(mut self) -> Self {
		self.rsx_to_html.no_slot_check = true;
		self
	}
}

impl RsxPipeline<RsxNode, Result<String>> for RsxToHtmlString {
	fn apply(self, root: RsxNode) -> Result<String> {
		root.bpipe(self.rsx_transforms)?
			.bref()
			.bpipe(self.rsx_to_html)
			.bpipe(self.render_html)
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
			.bpipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(1);
		let doc = rsx! { <MyComponent client:load /> }
			.bpipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(4);
	}
}
