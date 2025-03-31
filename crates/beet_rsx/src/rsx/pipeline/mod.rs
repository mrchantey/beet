mod build_step;
mod client_island;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod fs_src_pipeline;
mod register_effects;
mod slots_pipeline;
pub use register_effects::*;

pub use build_step::*;
pub use client_island::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use fs_src_pipeline::*;
pub use slots_pipeline::*;
#[cfg(feature = "css")]
mod scoped_style_pipeline;
#[cfg(feature = "css")]
pub use scoped_style_pipeline::*;

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
	fs_src: FsSrcPipeline,
	#[cfg(feature = "css")]
	scoped_style: ScopedStylePipeline,
	slots: SlotsPipeline,
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

		let mut client_directives = false;
		VisitRsxComponent::new(|c| {
			if c.template_directives.iter().any(|d| d.prefix == "client") {
				client_directives = true;
			}
		})
		.walk_node(&node);


		let mut doc = node
			.bref()
			.bpipe(self.rsx_to_html)
			.bpipe(self.html_to_document)?;
		if client_directives {
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
