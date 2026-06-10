//! A template-capable test world for the ui test suite. The `attr`/
//! `optional_attr` block-attribute helpers moved to `beet_core` (re-exported via
//! [`crate::node`]).
#[cfg(feature = "bsx")]
use crate::prelude::*;
use beet_core::prelude::*;

/// A [`World`] wired with the minimal plugins required to `spawn_template`: the
/// substrate's [`TemplatePlugin`], the [`DocumentPlugin`] templates lean on, and
/// (when `bsx` is enabled) the default BSX event/verb vocabulary
/// ([`BsxDefaultsPlugin`]) so a parsed `bx:click` resolves. Insert any required
/// resources before spawning.
#[cfg(feature = "bsx")]
pub fn test_world() -> World {
	(TemplatePlugin, DocumentPlugin, BsxDefaultsPlugin).into_world()
}

/// A [`World`] wired with the minimal plugins required to `spawn_template`: the
/// substrate's [`TemplatePlugin`] plus the [`DocumentPlugin`] templates lean on.
/// Insert any required resources before spawning.
#[cfg(not(feature = "bsx"))]
pub fn test_world() -> World {
	(TemplatePlugin, DocumentPlugin).into_world()
}
