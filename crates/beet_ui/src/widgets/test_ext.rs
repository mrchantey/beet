//! Shared harness for the widget tests: HTML and charcell renders through the
//! substrate, the worlds a control needs to run in, and (under `tui`) a live app
//! with the focus/keyboard/pointer drivers, so an interaction test drives the
//! same systems the real terminal does.
//!
//! Generic to the widget set. The helpers that know what a *schema* generated
//! live in [`schema_ui::test_ext`](super::schema_ui::test_ext), which layers
//! onto this one.
use crate::prelude::*;
use beet_core::prelude::*;

/// Render a template to an HTML string through the substrate.
pub fn render_html(
	template: impl bevy::ecs::template::Template<Output = ()>,
) -> String {
	let mut world = world_ext::ui_world();
	let root = world.spawn_template(template).unwrap().id();
	render_world(&mut world, root)
}

/// Render `root`'s existing subtree to HTML, for a world already built and
/// settled (an attribute mirrored from a synced [`Value`], say).
pub fn render_world(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

/// Render a template to plain charcell text in a `width`-column buffer, through
/// the same parse/measure/paint pipeline the terminal runs. `root_bundle` is
/// inserted on the built root, ie the [`Document`] its bindings resolve against.
pub fn render_charcell(
	width: u32,
	root_bundle: impl Bundle,
	template: impl bevy::ecs::template::Template<Output = ()>,
) -> String {
	let mut world = (
		TemplatePlugin,
		DocumentPlugin,
		CharcellPlugin,
		crate::style::material::MaterialStylePlugin::default(),
	)
		.into_world();
	let root = world.spawn_template(template).unwrap().id();
	world
		.entity_mut(root)
		.insert((root_bundle, FlexBuffer::new(width)));
	world.update_local();
	world.run_schedule(crate::parse::PostParseTree);
	world
		.entity_mut(root)
		.take::<FlexBuffer>()
		.unwrap()
		.render_plain()
}

/// A [`world_ext::ui_world`] twin with [`FormPlugin`] running, for renders that
/// assert on an attribute mirrored from a synced [`Value`] (`checked`). Settle
/// it with [`update_local`](WorldMutExt::update_local) before rendering.
pub fn form_world() -> World {
	(
		TemplatePlugin,
		DocumentPlugin,
		BsxDefaultsPlugin,
		FormPlugin,
	)
		.into_world()
}

/// A live app with the form controls' full driver stack: charcell parsing,
/// documents, focus/keyboard and [`FormPlugin`].
#[cfg(feature = "tui")]
pub fn form_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		bevy::input::InputPlugin,
		CharcellPlugin,
		RealtimeParsePlugin,
		DocumentPlugin,
		FocusPlugin,
		FormPlugin,
	));
	app
}

/// The entity of the first element with `tag`, in **document order**.
///
/// Not archetype order: a query iterates archetypes, whose order shifts with
/// component registration, so `element(app, "input")` meaning "the first input"
/// silently became "some input" the moment a plugin was added. Roots are visited
/// by entity id and each subtree depth-first in child order, which is the order
/// the renderers and the focus path use.
#[cfg(feature = "tui")]
pub fn element(app: &mut App, tag: &str) -> Entity {
	element_in(app.world_mut(), tag)
}

/// [`element`], for a world driven without an [`App`].
pub fn element_in(world: &mut World, tag: &str) -> Entity {
	elements_in(world, tag).into_iter().next().unwrap()
}

/// Every element with `tag` in document order, ie the several buttons a
/// generated collection control emits.
pub fn elements_in(world: &mut World, tag: &str) -> Vec<Entity> {
	let mut roots = world
		.query_filtered::<Entity, Without<ChildOf>>()
		.iter(world)
		.collect::<Vec<_>>();
	roots.sort();
	roots
		.into_iter()
		.flat_map(|root| find_elements(world, root, tag))
		.collect()
}

/// The descendants of `entity` (inclusive) whose element tag matches,
/// depth-first in child order.
fn find_elements(world: &World, entity: Entity, tag: &str) -> Vec<Entity> {
	let matched = world
		.get::<Element>(entity)
		.is_some_and(|element| element.tag() == tag)
		.then_some(entity);
	matched
		.into_iter()
		.chain(
			world
				.get::<Children>(entity)
				.into_iter()
				.flat_map(|children| children.iter())
				.flat_map(|child| find_elements(world, child, tag)),
		)
		.collect()
}

/// Run the frames a document-driven rebuild needs: the edit, the syncs it
/// dirties, the generation those spawn, and that generation's own first sync.
pub fn settle_world(world: &mut World) {
	for _ in 0..4 {
		world.update_local();
	}
}

/// Activate `entity`, the pointer half of the activation path, then settle.
pub fn click_world(world: &mut World, entity: Entity) {
	world.entity_mut(entity).trigger(PointerUp::new(entity));
	settle_world(world);
}

/// Focus the first element with `tag` on a fresh window surface, returning
/// `(window, entity)`. The per-surface focus path only delivers input to an
/// element scoped to the window it came from, as the real app is.
#[cfg(feature = "tui")]
pub fn focus_element(app: &mut App, tag: &str) -> (Entity, Entity) {
	let entity = element(app, tag);
	let window = app.world_mut().spawn_empty().id();
	app.world_mut()
		.entity_mut(entity)
		.insert((Focus, RenderSurface(window)));
	(window, entity)
}

/// Run enough frames for an edit to settle through the bidirectional document
/// sync: an input delivered in one frame is written back in the next, so an
/// assertion on the *document* needs more than the frame that typed it.
#[cfg(feature = "tui")]
pub fn settle(app: &mut App) {
	for _ in 0..3 {
		app.update();
	}
}

/// Type `text` into the focused element of `window`, one key message per char,
/// then settle.
#[cfg(feature = "tui")]
pub fn type_text(app: &mut App, window: Entity, text: &str) {
	use bevy::input::keyboard::Key;
	use bevy::input::keyboard::KeyCode;

	for ch in text.chars() {
		let ch = ch.to_string();
		app.world_mut().write_message(key_message(
			window,
			// the key code is unread by the text path, which reads `logical_key`
			KeyCode::KeyA,
			Key::Character(ch.as_str().into()),
			Some(ch.clone()),
		));
	}
	settle(app);
}

/// Press Enter on `window`, the activation and submit key, then settle.
#[cfg(feature = "tui")]
pub fn press_enter(app: &mut App, window: Entity) {
	use bevy::input::keyboard::Key;
	use bevy::input::keyboard::KeyCode;

	app.world_mut().write_message(key_message(
		window,
		KeyCode::Enter,
		Key::Enter,
		None,
	));
	settle(app);
}

/// One pressed-key message from `window`.
#[cfg(feature = "tui")]
fn key_message(
	window: Entity,
	key_code: bevy::input::keyboard::KeyCode,
	logical_key: bevy::input::keyboard::Key,
	text: Option<String>,
) -> bevy::input::keyboard::KeyboardInput {
	bevy::input::keyboard::KeyboardInput {
		key_code,
		logical_key,
		state: bevy::input::ButtonState::Pressed,
		text,
		repeat: false,
		window,
	}
}

/// Click `entity`, the pointer half of the activation path, then settle.
#[cfg(feature = "tui")]
pub fn click(app: &mut App, entity: Entity) {
	let pointer = app.world_mut().spawn_empty().id();
	app.world_mut()
		.entity_mut(entity)
		.trigger(PointerUp::new(pointer));
	settle(app);
}
