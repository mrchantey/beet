//! Which standard.site publication a page belongs to, and what its index
//! remembers of the page.
#[cfg(feature = "dag_cbor")]
use crate::prelude::*;
use beet_core::prelude::*;

/// A page's standard.site records as its publication's
/// [`StandardSiteIndex`] remembers them: the innermost publication at or above
/// the page, the page's document, and the `<link>`s naming both in its head
/// (see [`RouteHead`]). Nothing without an index, or without the data model
/// the records ride.
#[derive(SystemParam)]
pub(crate) struct StandardSiteQuery<'w, 's> {
	#[cfg(feature = "dag_cbor")]
	publications:
		Query<'w, 's, (&'static StandardSitePub, &'static StandardSiteIndex)>,
	// a build without the data model has no publication to find
	#[cfg(not(feature = "dag_cbor"))]
	_publications: Query<'w, 's, ()>,
}

#[cfg(feature = "dag_cbor")]
impl StandardSiteQuery<'_, '_> {
	/// The publication the page at route `page` belongs to and its index, the
	/// innermost where two nest.
	pub fn publication(
		&self,
		page: &RelPath,
	) -> Option<(&StandardSitePub, &StandardSiteIndex)> {
		self.publications
			.iter()
			.filter(|(publication, _)| {
				publication.document_path(page).is_some()
			})
			.max_by_key(|(publication, _)| publication.path.as_str().len())
	}

	/// The document the index remembers for the page at route `page`.
	pub fn document(
		&self,
		page: &RelPath,
	) -> Option<&StandardSiteIndexDocument> {
		let (publication, index) = self.publication(page)?;
		index.document(&publication.document_path(page)?)
	}
}

impl StandardSiteQuery<'_, '_> {
	/// The `(rel, href)` of each link the page at route `page` carries: the
	/// publication's on every page at or under its path, the document's on a
	/// page its index knows.
	pub fn links(&self, page: &RelPath) -> Vec<(&'static str, String)> {
		cfg_if! {
			if #[cfg(feature = "dag_cbor")] {
				let Some((_, index)) = self.publication(page) else {
					return Vec::new();
				};
				core::iter::once((
					"site.standard.publication",
					index.publication.uri.to_string(),
				))
				.chain(self.document(page).map(|document| {
					("site.standard.document", document.uri.to_string())
				}))
				.collect()
			} else {
				let _ = page;
				Vec::new()
			}
		}
	}
}
