//! Shared harness for the widget tests: HTML renders through the substrate,
//! and (under `tui`) a live app with the focus/keyboard/pointer drivers, so an
//! interaction test drives the same systems the real terminal does.
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

/// The one button that submits the form it sits in, ie the only one no
/// `type="button"` excludes ([`Button`]'s `action`).
pub fn submit_button(world: &mut World) -> Entity {
	let actions = world
		.query_once::<(&Attribute, &Value, &AttributeOf)>()
		.into_iter()
		.filter(|(attribute, value, _)| {
			attribute.as_str() == "type"
				&& value
					.as_str()
					.map(|value| value == "button")
					.unwrap_or_default()
		})
		.map(|(_, _, attribute_of)| **attribute_of)
		.collect::<HashSet<_>>();
	elements_in(world, "button")
		.into_iter()
		.find(|button| !actions.contains(button))
		.expect("no submit button")
}

/// The generated control bound to `path`, ie the leaf a form emitted for it.
pub fn bound(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &ResolvedFieldPath)>()
		.into_iter()
		.find(|(_, resolved)| resolved.field_path.to_string() == path)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no control is bound to `{path}`"))
}

/// The generated variant `<select>` choosing the enum at `path`, which binds no
/// field of its own (its value is the variant name).
pub fn variant_select(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &super::variant_select::VariantSelect)>()
		.into_iter()
		.find(|(_, select)| select.field.field_path.to_string() == path)
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no variant select chooses `{path}`"))
}

/// The generated add button of the collection control bound to `path`.
pub fn collection_add(world: &mut World, path: &str) -> Entity {
	world
		.query_once::<(Entity, &super::collection_edit::CollectionButton)>()
		.into_iter()
		.find(|(_, button)| {
			button.field.field_path.to_string() == path
				&& matches!(
					button.edit,
					super::collection_edit::CollectionEdit::Push(_)
						| super::collection_edit::CollectionEdit::Insert(_)
				)
		})
		.map(|(entity, _)| entity)
		.unwrap_or_else(|| panic!("no collection add button edits `{path}`"))
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
