//! Minimal [`World`](beet_core::prelude::World) constructors for UI templates.
use crate::prelude::*;
use beet_core::prelude::*;

/// A [`World`] wired with the minimal plugins required to `spawn_template` and
/// render, see [`ui_app`]. Insert any required resources before spawning.
pub fn ui_world() -> World { core::mem::take(ui_app().world_mut()) }

/// The [`App`] a [`ui_world`] is taken from, for a caller adding its own
/// templates first (`ui_app().install_type::<Badge>()`): the substrate's
/// [`TemplatePlugin`], the [`DocumentPlugin`] templates lean on, the built-in
/// render targets ([`RenderPlugin`]) and (when `bsx` is enabled) the default
/// BSX event vocabulary ([`BsxDefaultsPlugin`]) so a parsed `bx:click`
/// resolves.
pub fn ui_app() -> App {
	let mut app = App::new();
	app.add_plugins((TemplatePlugin, DocumentPlugin, RenderPlugin));
	#[cfg(feature = "bsx")]
	app.add_plugins(BsxDefaultsPlugin);
	app
}
