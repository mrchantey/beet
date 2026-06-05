//! The welcome page shown when the cwd has no `beet.json`.

use beet::prelude::*;

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

/// Spawn, render and print [`SceneNotFound`] through the charcell pipeline,
/// despawning the ephemeral render tree afterwards.
pub fn render_scene_not_found(world: &mut World) -> Result {
	let root = world.spawn_scene(SceneNotFound::scene(default()))?.id();
	let output = AnsiTermRenderer::new()
		.render(&mut RenderContext::new(root, world))?
		.to_string();
	world.entity_mut(root).despawn();
	cross_log_noline!("{output}");
	Ok(())
}
