//! Harness for the ui tests: the live-TUI render stack minus the terminal,
//! a page bound to a host surface as `page_host`'s own tests bind one.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;
use beet_ui::prelude::CharcellPlugin;
use beet_ui::prelude::DoubleBuffer;
use beet_ui::prelude::Portal;
use beet_ui::prelude::RealtimeParsePlugin;
use beet_ui::prelude::RenderSurface;
use bevy::math::UVec2;

/// An app with the live-TUI render stack, the thread plugins and their ui.
pub(super) fn render_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TemplatePlugin,
		DocumentPlugin,
		CharcellPlugin,
		RealtimeParsePlugin,
		PageHostPlugin,
	))
	.install_type::<(ThreadPlugin, ThreadUiPlugin)>();
	app
}

/// Spawn `page` as the page of a fresh host `size` cells large, answering
/// the host whose buffer is painted and the page root.
pub(super) fn mount_page(
	app: &mut App,
	size: UVec2,
	page: Snippet,
) -> (Entity, Entity) {
	let host = app.world_mut().spawn(PageHost::bundle(size)).id();
	let page = app.world_mut().spawn_template(page).unwrap().id();
	// bind the page to the host surface (`bind_surface_page` is
	// router-internal; a fresh host has no outgoing page to clean up). The
	// slot is a descendant of the host, a grandchild as `PageHost::bundle`
	// spawns it.
	let slot = app
		.world_mut()
		.with_state::<Query<&Children>, _>(|children| {
			children.iter_descendants(host).collect::<Vec<_>>()
		})
		.into_iter()
		.find(|entity| app.world().get::<PageSlot>(*entity).is_some())
		.unwrap();
	app.world_mut().entity_mut(page).insert(RenderSurface(host));
	app.world_mut().entity_mut(slot).insert(Portal::new(page));
	(host, page)
}

/// A few frames, enough for the projection, the document chain and the
/// paint to settle.
pub(super) fn settle(app: &mut App) { (0..4).for_each(|_| app.update()); }

/// The host's painted frame as plain text.
pub(super) fn frame(app: &App, host: Entity) -> String {
	app.world()
		.get::<DoubleBuffer>(host)
		.unwrap()
		.current_buffer()
		.render_plain()
}
