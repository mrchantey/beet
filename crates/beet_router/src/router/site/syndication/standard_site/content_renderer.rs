//! The formats a standard site document's `content` slot carries.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use core::fmt;

/// A `content` format: renders a page into the object a standard site
/// document's `content` slot carries, so a reader that knows the format shows
/// the post natively without visiting the site.
///
/// Pure over the page it is handed, which already carries everything the
/// format embeds, so a render is deterministic and a dry run renders exactly
/// what a publish would write. Registered by its NSID in
/// [`StandardSiteContentRenderers`], which writes the `$type`.
pub trait StandardSiteContentRenderer: 'static + Send + Sync {
	/// The def the rendered object implements, ie `pub.leaflet.content`.
	fn nsid(&self) -> Nsid;

	/// The object's fields for `page`, an object without its `$type`.
	fn render(&self, page: &BuiltPage) -> Result<Value>;
}

/// What a [`StandardSiteContentRenderer`] is handed: a listed page and its
/// parsed article, the body without the chrome a syndicated copy should not
/// carry.
///
/// The parsed article rather than the live scene, since a page renders by an
/// in-process request that answers bytes, and a renderer walking only the
/// article it was given cannot syndicate chrome by accident.
#[derive(Debug, Clone)]
pub struct BuiltPage {
	/// Where the page serves and what it was authored with.
	pub page: SyndicationPage,
	/// The page's article, parsed.
	pub article: Vec<BsxNode>,
}

/// The `content` formats this runtime can render, by the NSID a publication
/// declares: a name with no renderer is an error listing the registered ones,
/// so a declaration says exactly which object lands in each record.
///
/// Cloned out of the world by a publish step, which renders off it.
#[derive(Default, Clone, Resource)]
pub struct StandardSiteContentRenderers(
	HashMap<Nsid, Arc<dyn StandardSiteContentRenderer>>,
);

impl fmt::Debug for StandardSiteContentRenderers {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.debug_set().entries(self.nsids()).finish()
	}
}

impl StandardSiteContentRenderers {
	/// Register `renderer` under its NSID, replacing any registered there.
	pub fn register(
		&mut self,
		renderer: impl StandardSiteContentRenderer,
	) -> &mut Self {
		self.0.insert(renderer.nsid(), Arc::new(renderer));
		self
	}

	/// The registered NSIDs, sorted.
	pub fn nsids(&self) -> Vec<&Nsid> {
		let mut nsids = self.0.keys().collect::<Vec<_>>();
		nsids.sort();
		nsids
	}

	/// The renderer registered under `nsid`.
	///
	/// # Errors
	/// Errors when none is, listing the registered NSIDs.
	pub fn get(&self, nsid: &Nsid) -> Result<&dyn StandardSiteContentRenderer> {
		match self.0.get(nsid) {
			Some(renderer) => renderer.as_ref().xok(),
			None => bevybail!(
				"no standard site content renderer is registered for \
				 `{nsid}`, the registered formats are: [{}]",
				self.nsids()
					.iter()
					.map(|nsid| nsid.as_str())
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}

	/// `page` rendered by the renderer registered under `nsid`, as the
	/// union member a document's `content` carries.
	pub fn render(&self, nsid: &Nsid, page: &BuiltPage) -> Result<OpenUnion> {
		OpenUnion::new(nsid.clone(), self.get(nsid)?.render(page)?)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A format carrying the article's text alone.
	struct PlainText;

	impl StandardSiteContentRenderer for PlainText {
		fn nsid(&self) -> Nsid { Nsid::new_static("org.beet.test.plaintext") }

		fn render(&self, page: &BuiltPage) -> Result<Value> {
			fn push_text(nodes: &[BsxNode], text: &mut String) {
				for node in nodes {
					match node {
						BsxNode::Text(run) => text.push_str(run),
						BsxNode::Element(element) => {
							push_text(&element.children, text)
						}
						_ => {}
					}
				}
			}
			let mut text = String::new();
			push_text(&page.article, &mut text);
			value!({ "text": text }).xok()
		}
	}

	fn page() -> BuiltPage {
		BuiltPage {
			page: SyndicationPage {
				path: RelPath::new("blog/post"),
				meta: default(),
			},
			article: BsxNode::parse_document(
				"<p>Hello <em>world</em></p>",
				&BsxParseConfig::html(),
			)
			.unwrap(),
		}
	}

	/// A registered format renders as the member its NSID names.
	#[beet_core::test]
	fn renders_a_registered_format() {
		let mut renderers = StandardSiteContentRenderers::default();
		renderers.register(PlainText);
		let content = renderers
			.render(&Nsid::new_static("org.beet.test.plaintext"), &page())
			.unwrap();
		content
			.r#type()
			.as_str()
			.xpect_eq("org.beet.test.plaintext");
		content
			.fields()
			.get("text")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Hello world");
	}

	/// An unregistered format fails naming the ones that are.
	#[beet_core::test]
	fn refuses_an_unregistered_format() {
		let mut renderers = StandardSiteContentRenderers::default();
		renderers.register(PlainText);
		renderers
			.render(&Nsid::new_static("pub.leaflet.content"), &page())
			.unwrap_err()
			.to_string()
			.xpect_contains("pub.leaflet.content")
			.xpect_contains("[org.beet.test.plaintext]");
	}
}
