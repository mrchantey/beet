//! Module containing pipelines to be applied to an [`RsxRoot`]
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

// pub trait PipingHot: Sized {
// 	fn pipe<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }
// }
// impl<T: Sized> PipingHot for T {}

/// Trait for pipelines that will mutate an [`RsxPluginTarget`]
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


/// Blanket implementation for all types allowing for method-chaining.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait RsxPipelineTarget: Sized {
	fn bmap<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }

	fn bpipe<P: RsxPipeline<Self, O>, O>(self, pipeline: P) -> O {
		pipeline.apply(self)
	}
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

impl RsxPipeline<RsxRoot, Result<RsxRoot>> for DefaultRsxTransforms {
	fn apply(self, root: RsxRoot) -> Result<RsxRoot> {
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

impl RsxPipeline<RsxRoot, Result<HtmlDocument>> for RsxToHtmlDocument {
	fn apply(self, mut root: RsxRoot) -> Result<HtmlDocument> {
		root = root.bpipe(self.rsx_transforms)?;

		let mut client_directives = false;
		VisitRsxComponent::new(|c| {
			if c.template_directives.iter().any(|d| d.prefix == "client") {
				client_directives = true;
			}
		})
		.walk_node(&root.node);

		let mut doc = root
			.as_ref()
			.bpipe(self.rsx_to_html)
			.bpipe(self.html_to_document)?;
		if client_directives {
			doc = doc
				.bmap(|doc| (doc, root.as_ref()))
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

impl RsxPipeline<RsxRoot, Result<String>> for RsxToHtmlString {
	fn apply(self, root: RsxRoot) -> Result<String> {
		root.bpipe(self.rsx_transforms)?
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
	fn my_component(_: MyComponent) -> RsxRoot {
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
