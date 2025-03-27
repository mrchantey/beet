//! Module containing pipelines to be applied to an [`RsxRoot`]
mod build_step;
mod client_island;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod fs_src_pipeline;
mod register_effects;
mod slots_pipeline;
pub use register_effects::*;
use std::pin::Pin;

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



/// Trait for pipelines that will mutate an [`RsxPluginTarget`]
pub trait RsxPipeline<In: RsxPipelineTarget, Out: RsxPipelineTarget = In> {
	/// Consume self and apply to the target
	fn apply(self, value: In) -> Out;
}

impl<F, In: RsxPipelineTarget, Out: RsxPipelineTarget> RsxPipeline<In, Out>
	for F
where
	F: FnOnce(In) -> Out,
{
	fn apply(self, value: In) -> Out { self(value) }
}

pub trait RsxPipelineTarget: Sized {
	fn pipe<P: RsxPipeline<Self, O>, O: RsxPipelineTarget>(
		self,
		pipeline: P,
	) -> O {
		pipeline.apply(self)
	}
	fn pipe_with<
		P: RsxPipeline<(Self, T2), O>,
		O: RsxPipelineTarget,
		T2: RsxPipelineTarget,
	>(
		self,
		other: T2,
		pipeline: P,
	) -> O
	where
		(Self, T2): RsxPipelineTarget,
	{
		pipeline.apply((self, other))
	}
}
pub trait RsxPipelineTargetIter<T: RsxPipelineTarget>:
	Sized + IntoIterator<Item = T>
{
	fn pipe_each<P: RsxPipeline<T, O> + Clone, O: RsxPipelineTarget>(
		self,
		pipeline: P,
	) -> Vec<O> {
		self.into_iter()
			.map(|v| pipeline.clone().apply(v))
			.collect()
	}
}
impl<T: IntoIterator<Item = U>, U: RsxPipelineTarget> RsxPipelineTargetIter<U>
	for T
{
}


impl<T: RsxPipelineTarget> RsxPipelineTarget for &T {}
impl<T: RsxPipelineTarget> RsxPipelineTarget for Option<T> {}
impl<T: RsxPipelineTarget> RsxPipelineTarget for Result<T> {}
impl<T: RsxPipelineTarget> RsxPipelineTarget for Vec<T> {}
impl<T: RsxPipelineTarget> RsxPipelineTarget for Box<T> {}
impl<T: RsxPipelineTarget> RsxPipelineTarget
	for Pin<Box<dyn Future<Output = T>>>
{
}
impl<T: RsxPipelineTarget> RsxPipelineTarget
	for Box<dyn Fn() -> Pin<Box<dyn Future<Output = T>>>>
{
}


impl<T1: RsxPipelineTarget, T2: RsxPipelineTarget> RsxPipelineTarget
	for (T1, T2)
{
}



impl RsxPipelineTarget for () {}
impl RsxPipelineTarget for String {}
impl RsxPipelineTarget for RsxRoot {}
impl RsxPipelineTarget for RsxNode {}

impl RsxPipelineTarget for RsxElement {}
impl RsxPipelineTarget for RsxBlock {}
impl RsxPipelineTarget for RsxComponent {}

impl RsxPipelineTarget for HtmlNode {}
impl RsxPipelineTarget for HtmlDocument {}


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
		let root = root.pipe(self.fs_src)?;
		#[cfg(feature = "css")]
		let root = root.pipe(self.scoped_style)?;
		let root = root.pipe(self.slots)?;
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
		root = root.pipe(self.rsx_transforms)?;

		let mut client_directives = false;
		VisitRsxComponent::new(|c| {
			if c.template_directives.iter().any(|d| d.prefix == "client") {
				client_directives = true;
			}
		})
		.walk_node(&root.node);

		let mut doc = root
			.as_ref()
			.pipe(self.rsx_to_html)
			.pipe(self.html_to_document)?;
		if client_directives {
			doc = doc.pipe_with(root.as_ref(), self.html_doc_to_resumable);
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
		root.pipe(self.rsx_transforms)?
			.pipe(self.rsx_to_html)
			.pipe(self.render_html)
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
			.pipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(1);
		let doc = rsx! { <MyComponent client:load /> }
			.pipe(RsxToHtmlDocument::default())
			.unwrap();
		expect(doc.body.len()).to_be(4);
	}
}
