//! The `<link>`s a page's head names its standard.site records with.
#[cfg(feature = "dag_cbor")]
use crate::prelude::*;
use beet_core::prelude::*;

/// The standard.site `<link>`s of a page's head, see [`RouteHead`]: the
/// publication's on every page at or under its path, and the document's on a
/// page its [`StandardSiteIndex`] knows, the innermost publication winning
/// where two nest. Nothing without an index, or without the data model the
/// records ride.
#[derive(SystemParam)]
pub(crate) struct StandardSiteLinkQuery<'w, 's> {
	#[cfg(feature = "dag_cbor")]
	publications:
		Query<'w, 's, (&'static StandardSitePub, &'static StandardSiteIndex)>,
	// a build without the data model has no publication to link
	#[cfg(not(feature = "dag_cbor"))]
	_publications: Query<'w, 's, ()>,
}

impl StandardSiteLinkQuery<'_, '_> {
	/// The `(rel, href)` of each link the page at route `page` carries.
	pub fn links(&self, page: &RelPath) -> Vec<(&'static str, String)> {
		cfg_if! {
			if #[cfg(feature = "dag_cbor")] {
				let Some((publication, index)) = self
					.publications
					.iter()
					.filter(|(publication, _)| {
						publication.document_path(page).is_some()
					})
					.max_by_key(|(publication, _)| publication.path.as_str().len())
				else {
					return Vec::new();
				};
				let document = publication
					.document_path(page)
					.and_then(|path| index.document(&path).cloned());
				core::iter::once((
					"site.standard.publication",
					index.publication.uri.to_string(),
				))
				.chain(document.map(|document| {
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
