//! `<StandardSitePub/>`: the declaration a publication is built from.
use beet_core::prelude::*;

/// A route subtree published as a standard.site publication: what the
/// publication record says about itself, and where its documents come from.
///
/// The documents are the listed dated pages beneath [`path`](Self::path),
/// the set an `<RssFeed/>` declared there carries, so the declaration names no
/// posts.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct StandardSitePub {
	/// The route path the publication roots at, ie `blog`, joined to the
	/// site's homepage to form the publication's url. Empty for a whole site.
	pub path: RelPath,
	/// The publication's name.
	pub name: SmolStr,
	/// What the publication is about.
	pub description: Option<String>,
	/// The self labels the publication carries, normally none.
	pub labels: Vec<SmolStr>,
	/// Whether the publication may appear in discovery feeds.
	pub show_in_discover: bool,
}

impl Default for StandardSitePub {
	fn default() -> Self {
		Self {
			path: default(),
			name: default(),
			description: None,
			labels: Vec::new(),
			// the lexicon's own default
			show_in_discover: true,
		}
	}
}
