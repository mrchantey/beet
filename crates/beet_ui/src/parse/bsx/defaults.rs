//! The default BSX event registration.
//!
//! Core keeps the [`EventRegistry`] empty: it knows no concrete event, and no
//! way to evaluate a script, so neither bevy picking nor a JS engine enters it.
//! This plugin supplies the concrete `click` installer, so every
//! `bx:click="await target.set_field('count', 1)"` works. An app that wants a
//! different trigger set registers its own instead of (or alongside) this one.
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the default BSX event vocabulary into the core seam, plus the
/// widget set by name so a `<Head/>`/`<Sidebar/>` tag resolves.
///
/// Builds on [`BsxPlugin`] (which seeds the empty registry). With the
/// `scripting` feature it also installs the `click` installer and the
/// [`AsyncPlugin`] its evaluations resolve against; without it a `bx:<event>`
/// directive still builds, it just carries no behavior.
#[derive(Default)]
pub struct BsxDefaultsPlugin;

impl Plugin for BsxDefaultsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BsxPlugin, widget_plugin));
		#[cfg(feature = "scripting")]
		{
			// every `world` call an event script makes is served at the async
			// sync point, so the seat has to exist before one can fire.
			app.init_plugin::<AsyncPlugin>();
			super::event_script::register_event_scripts(app.world_mut());
		}
	}
}
