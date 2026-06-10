//! Live route rendering: paint the active route tree into a persistent
//! [`DoubleBuffer`] and re-render on navigation.
//!
//! The one-shot CLI path serializes a route's template tree to a string and
//! despawns it. The live TUI instead keeps the rendered tree alive and paints it
//! into a persistent [`DoubleBuffer`] each frame (via [`RealtimeParsePlugin`]),
//! re-rendering when the active [`CurrentScene`] changes. The injected
//! difference is exactly the buffer target plus the persistent lifecycle, not a
//! forked render path: the scene tree is still built through the template
//! substrate, and the charcell pipeline still walks it (here by reference, via a
//! [`RenderRef`] slot under the buffer host).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;
use bevy::math::UVec2;

/// A live-render host: a [`DoubleBuffer`] plus the [`RenderRef`] slot that
/// transcludes the active [`CurrentScene`] into it.
///
/// Spawn one with [`live_scene_host`]. Mark a built route tree [`CurrentScene`]
/// and [`sync_live_scene`] points the slot at it, so the charcell pipeline paints
/// it into the buffer. Navigating swaps `CurrentScene`, which re-points the slot
/// and repaints.
#[derive(Component)]
pub struct LiveSceneHost;

/// The slot entity (a child of the host) whose [`RenderRef`] transcludes the
/// current scene. Kept distinct from the host so the host's buffer renders the
/// slot, and the slot's reference can be retargeted without touching the buffer.
#[derive(Component)]
pub struct LiveSceneSlot;

/// Spawn a live-render host: a `size`-cell [`DoubleBuffer`] with a single
/// [`RenderRef`] slot child, ready for [`sync_live_scene`] to point at the
/// active [`CurrentScene`].
pub fn live_scene_host(size: UVec2) -> impl Bundle {
	(
		LiveSceneHost,
		DoubleBuffer::new(size),
		children![(LiveSceneSlot, RenderRef::new(Entity::PLACEHOLDER))],
	)
}

/// Registers the live-render sync system.
///
/// Pairs with [`CharcellPlugin`] + [`RealtimeParsePlugin`] (the repaint loop) and
/// [`NavigatorPlugin`] (which marks the navigated scene [`CurrentScene`]).
#[derive(Default)]
pub struct LiveScenePlugin;

impl Plugin for LiveScenePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, sync_live_scene);
	}
}

/// ECS system: point each host's [`RenderRef`] slot at the active
/// [`CurrentScene`], so the buffer paints the current route.
///
/// Runs when a new `CurrentScene` is added (navigation) and retargets the slot;
/// the next [`RealtimeParsePlugin`] repaint walks the new scene through the
/// reference. A no-op when nothing changed.
pub fn sync_live_scene(
	scenes: Populated<Entity, Added<CurrentScene>>,
	mut slots: Query<&mut RenderRef, With<LiveSceneSlot>>,
) {
	let Some(scene) = scenes.iter().next() else {
		return;
	};
	for mut slot in slots.iter_mut() {
		slot.set_if_neq(RenderRef::new(scene));
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::math::UVec2;

	/// The live-TUI render stack minus the terminal host: charcell pipeline,
	/// per-frame repaint, the document chain, and the live-scene sync.
	fn live_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			LiveScenePlugin,
		));
		app
	}

	/// Build a page tree marked as the active scene, returning its root entity.
	///
	/// Built through the template substrate (`spawn_template` + `snippet`) so a
	/// page of `#[template]` widgets resolves its slots/lifecycle, exactly as the
	/// route constructors build per-request content.
	fn spawn_scene(app: &mut App, bundle: impl Bundle) -> Entity {
		let scene = app.world_mut().spawn_template(snippet(bundle)).id();
		app.world_mut().entity_mut(scene).insert(CurrentScene);
		scene
	}

	/// The host buffer's painted frame as plain text after one frame.
	fn frame(app: &mut App, host: Entity) -> String {
		// one frame: PreUpdate points the slot at CurrentScene, then the post-parse
		// pipeline paints the host buffer through the RenderRef slot.
		app.update();
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.current_buffer()
			.render_plain()
	}

	/// The active route renders into the persistent buffer, and navigating to a
	/// second route re-renders it (the previous scene is dropped).
	#[beet_core::test]
	fn renders_and_re_renders_active_scene() {
		let mut app = live_app();
		let host =
			app.world_mut().spawn(live_scene_host(UVec2::new(40, 8))).id();

		// initial route: Alpha
		let alpha = spawn_scene(&mut app, rsx! { <p>"Alpha page"</p> });
		frame(&mut app, host).xpect_contains("Alpha page");

		// navigate: a new scene becomes current; the slot re-points and repaints.
		// the previous scene leaves the active set (the single-scene observer would
		// despawn it in the full app; here we drop it explicitly).
		app.world_mut().entity_mut(alpha).remove::<CurrentScene>();
		let _beta = spawn_scene(&mut app, rsx! { <p>"Beta page"</p> });
		let out = frame(&mut app, host);
		out.as_str().xpect_contains("Beta page");
		out.xnot().xpect_contains("Alpha page");
	}
}
