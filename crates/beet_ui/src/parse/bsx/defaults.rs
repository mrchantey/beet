//! The default BSX event handlers.
//!
//! Core rewrites `bx:on<event>` to a `<script {OnEvent}>` child and knows no
//! concrete event, and no way to evaluate a script, so neither bevy picking nor a JS
//! engine enters it. This plugin runs the `click` handlers, so every
//! `bx:onclick="await target.set_field('count', 1)"` works. An app that wants
//! another trigger adds its own observer beside this one.
use crate::prelude::*;
use beet_core::prelude::*;

/// Runs the default BSX event handlers, and registers the widget set by name
/// so a `<Head/>`/`<Sidebar/>` tag resolves.
///
/// Builds on [`BsxPlugin`]. With the `scripting` feature it also runs `click`
/// handlers and installs the [`AsyncPlugin`] their evaluations resolve
/// against; without it a `bx:on<event>` directive still builds, its handler
/// inert.
#[derive(Default)]
pub struct BsxDefaultsPlugin;

impl Plugin for BsxDefaultsPlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<BsxPlugin>().add_plugins(widget_plugin);
		#[cfg(feature = "scripting")]
		{
			// every `world` call an event script makes is served at the async
			// sync point, so the seat has to exist before one can fire.
			app.install_type::<AsyncPlugin>()
				.add_observer(super::event_script::run_click_scripts);
		}
	}
}
