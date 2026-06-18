//! The site root: the directory that site-relative paths resolve against.
//!
//! Backs both the native directory-scanning [`RoutesDir`](crate::prelude::RoutesDir)
//! and the cross-platform `<Template src>` include. Just a path, so it lives here
//! (always compiled) rather than in the native-only `routes_dir` module.

use beet_core::prelude::*;
use std::path::Path;

/// The directory site-relative paths resolve against: a `<RoutesDir src>` scan
/// root and a `<Template src>` include base. A host sets this to the directory of
/// its entry file; defaults to the workspace root, matching the [`WsPathBuf`]
/// convention. 
#[derive(Debug, Clone, Resource)]
pub struct SiteRoot(pub AbsPathBuf);

impl SiteRoot {
	/// A site root at `path`, workspace-relative.
	pub fn new_workspace_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		AbsPathBuf::new_workspace_rel(path).map(Self)
	}
}

impl Default for SiteRoot {
	fn default() -> Self {
		Self(AbsPathBuf::new_workspace_rel("").expect("workspace root exists"))
	}
}
