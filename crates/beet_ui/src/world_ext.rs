//! Minimal [`World`](beet_core::prelude::World) constructors for UI templates.

/// A [`World`](beet_core::prelude::World) wired with the minimal plugins required
/// to `spawn_template`: the substrate's
/// [`TemplatePlugin`](beet_core::prelude::TemplatePlugin), the
/// [`DocumentPlugin`](beet_core::prelude::DocumentPlugin) templates lean on, and
/// (when `bsx` is enabled) the default BSX event/verb vocabulary
/// ([`BsxDefaultsPlugin`](crate::prelude::BsxDefaultsPlugin)) so a parsed
/// `bx:click` resolves. Insert any required resources before spawning.
#[cfg(feature = "bsx")]
pub fn ui_world() -> beet_core::prelude::World {
	use crate::prelude::*;
	use beet_core::prelude::*;
	(TemplatePlugin, DocumentPlugin, BsxDefaultsPlugin).into_world()
}

/// See [`ui_world`]; this variant omits the BSX vocabulary when `bsx` is off.
#[cfg(not(feature = "bsx"))]
pub fn ui_world() -> beet_core::prelude::World {
	use beet_core::prelude::*;
	(TemplatePlugin, DocumentPlugin).into_world()
}
