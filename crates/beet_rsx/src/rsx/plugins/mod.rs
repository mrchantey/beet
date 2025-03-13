//! Module containing all plugins to be applied to an [`RsxRoot`]
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod fs_src_plugin;
mod slots_plugin;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use fs_src_plugin::*;
pub use slots_plugin::*;
#[cfg(feature = "css")]
mod scoped_style_plugin;
#[cfg(feature = "css")]
pub use scoped_style_plugin::*;

use crate::prelude::*;
use anyhow::Result;


#[derive(Default)]
pub struct DefaultRsxTransforms {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	fs_src: FsSrcPlugin,
	#[cfg(feature = "css")]
	scoped_style: ScopedStylePlugin,
	slots: SlotsPlugin,
}

impl RsxPlugin<RsxRoot> for DefaultRsxTransforms {
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
}

impl RsxPlugin<RsxRoot, HtmlDocument> for RsxToHtmlDocument {
	fn apply(self, root: RsxRoot) -> Result<HtmlDocument> {
		Ok(root
			.pipe(self.rsx_transforms)?
			.pipe(self.rsx_to_html)?
			.pipe1(self.html_to_document)?
			.take1())
	}
}


#[derive(Default)]
pub struct RsxToHtmlString {
	pub rsx_transforms: DefaultRsxTransforms,
	pub rsx_to_html: RsxToHtml,
	pub render_html: RenderHtml,
}

impl RsxPlugin<RsxRoot, String> for RsxToHtmlString {
	fn apply(self, root: RsxRoot) -> Result<String> {
		Ok(root
			.pipe(self.rsx_transforms)?
			.pipe(self.rsx_to_html)?
			.pipe1(self.render_html)?
			.take1())
	}
}
#[derive(Default)]
pub struct RsxToHtmlDocumentString {
	pub rsx_transforms: DefaultRsxTransforms,
	pub rsx_to_html: RsxToHtml,
	pub html_to_document: HtmlToDocument,
	pub render_html: RenderHtml,
}

impl RsxPlugin<RsxRoot, String> for RsxToHtmlDocumentString {
	fn apply(self, root: RsxRoot) -> Result<String> {
		Ok(root
			.pipe(self.rsx_transforms)?
			.pipe(self.rsx_to_html)?
			.pipe1(self.html_to_document)?
			.pipe1(self.render_html)?
			.take1())
	}
}


/// Trait for plugins that will mutate an [`RsxPluginTarget`]
pub trait RsxPlugin<In: RsxPluginTarget, Out: RsxPluginTarget = In> {
	/// Consume self and apply to the target
	fn apply(self, value: In) -> Result<Out>;
}

impl<F, In: RsxPluginTarget, Out: RsxPluginTarget> RsxPlugin<In, Out> for F
where
	F: FnOnce(In) -> Result<Out>,
{
	fn apply(self, value: In) -> Result<Out> { self(value) }
}

// impl RsxRoot {
// 	/// Apply default rsx plugins:
// 	/// - [FsSrcPlugin]
// 	/// - [ScopedStylePlugin]
// 	/// - [SlotsPlugin]
// 	pub fn apply_default_plugins(&mut self) -> Result<()> {
// 		FsSrcPlugin::default().apply(self)?;
// 		ScopedStylePlugin::default().apply(self)?;
// 		SlotsPlugin::default().apply(self)?;
// 		Ok(())
// 	}
// }


pub trait RsxPluginTarget: Sized {
	fn pipe<P: RsxPlugin<Self, O>, O: RsxPluginTarget>(
		self,
		plugin: P,
	) -> Result<O> {
		plugin.apply(self.into())
	}
}


pub trait RsxPluginTargetTuple<P0: RsxPluginTarget, P1: RsxPluginTarget> {
	fn pipe0<P: RsxPlugin<P0, O>, O: RsxPluginTarget>(
		self,
		plugin: P,
	) -> Result<(O, P1)>;
	fn pipe1<P: RsxPlugin<P1, O>, O: RsxPluginTarget>(
		self,
		plugin: P,
	) -> Result<(P0, O)>;
	fn take0(self) -> P0;
	fn take1(self) -> P1;
}
impl<P0: RsxPluginTarget, P1: RsxPluginTarget> RsxPluginTargetTuple<P0, P1>
	for (P0, P1)
{
	fn take0(self) -> P0 { self.0 }
	fn take1(self) -> P1 { self.1 }
	fn pipe0<P: RsxPlugin<P0, O>, O: RsxPluginTarget>(
		self,
		plugin: P,
	) -> Result<(O, P1)> {
		Ok((self.0.pipe(plugin)?, self.1))
	}

	fn pipe1<P: RsxPlugin<P1, O>, O: RsxPluginTarget>(
		self,
		plugin: P,
	) -> Result<(P0, O)> {
		Ok((self.0, self.1.pipe(plugin)?))
	}
}

impl<T: RsxPluginTarget> RsxPluginTarget for &T {}

impl RsxPluginTarget for String {}
impl RsxPluginTarget for RsxRoot {}
impl RsxPluginTarget for RsxNode {}
impl RsxPluginTarget for HtmlNode {}
impl RsxPluginTarget for Vec<HtmlNode> {}
impl RsxPluginTarget for HtmlDocument {}
impl RsxPluginTarget for (RsxRoot, Vec<HtmlNode>) {}
impl RsxPluginTarget for (RsxRoot, HtmlDocument) {}
impl RsxPluginTarget for (&RsxRoot, Vec<HtmlNode>) {}
impl RsxPluginTarget for (&RsxRoot, HtmlDocument) {}
impl RsxPluginTarget for (RsxNode, Vec<HtmlNode>) {}
impl RsxPluginTarget for (RsxNode, HtmlDocument) {}
impl RsxPluginTarget for (&RsxNode, Vec<HtmlNode>) {}
impl RsxPluginTarget for (&RsxNode, HtmlDocument) {}
