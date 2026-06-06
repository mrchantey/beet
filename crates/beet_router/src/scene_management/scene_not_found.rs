//! The welcome page shown when the cwd has no `beet.json`.

use crate::prelude::*;
use beet_core::prelude::*;

/// Rendered when no `beet.json` is found in the cwd: like a game engine pressing
/// play with no scene loaded, the CLI has no behaviour of its own.
#[scene]
pub fn SceneNotFound() -> impl Scene {
	rsx! {
		<div>
			<h1>"welcome to beet!"</h1>
			<p>"Beet is an open world app, providing capabilities only."</p>
			<p>"Please add a `beet.json` to this directory to define behavior."</p>
			<p>"Find out more at "<a href="https://beet.org">"beet.org"</a></p>
		</div>
	}
}

/// The root route serving [`SceneNotFound`], spawned by the host when the cwd
/// has no `beet.json` so `beet` answers with the welcome page through the normal
/// request pipeline (content-negotiated, like any other route) instead of
/// logging and exiting.
pub fn scene_not_found_route() -> impl Bundle {
	render_action::scene_func_route("", SceneNotFound::scene)
}
