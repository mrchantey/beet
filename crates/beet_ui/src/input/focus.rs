//! The focus model, renderer-agnostic.
//!
//! [`Focus`] marks the single focused entity per surface; [`Focusable`] marks
//! elements that can take focus (`<input>`/`<button>`/`<a>`/`<textarea>`,
//! inferred from the tag). Clicking a focusable focuses it
//! ([`focus_on_click`]) and the focused entity carries the
//! [`Focused`](crate::prelude::ElementState::Focused) state so
//! `:focus`/`:focus-visible` rules apply.
//!
//! Which input drives it is the renderer's: the terminal routes its keys
//! through the `keyboard` half (`input/keyboard.rs`: `Tab` traversal, `Enter`
//! activation, text entry into the focused [`Value`]), while the browser's
//! native controls own their keys and the DOM input mirrors the document's
//! focus into this model (`focusin` inserts, `focusout` removes).
use crate::prelude::ElementState;
use crate::prelude::ElementStateMap;
use crate::prelude::PointerDown;
#[cfg(feature = "template")]
use crate::prelude::Submit;
use crate::prelude::SurfaceQuery;
use beet_core::prelude::*;

/// Marker for the focused entity that receives keyboard input on a surface.
///
/// At most one entity carries `Focus` *per surface* (per
/// [`RenderSurface`](crate::prelude::RenderSurface)): the
/// `on_add` hook clears `Focus` from every other entity on the same surface, so
/// each session (one per SSH connection) keeps its own focused element. Having no
/// focused entity is a valid steady state.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
#[component(on_add = Self::on_add)]
pub struct Focus;

impl Focus {
	/// Clears `Focus` from other entities on the same surface so only the newest
	/// one keeps it, leaving other surfaces' focus untouched.
	///
	/// A [`DeferredWorld`] cannot run an arbitrary query inline, so the
	/// full-world work is queued as a command closure.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let added = cx.entity;
		world.commands().queue(move |world: &mut World| {
			let stale = world
				.with_state::<(Query<Entity, With<Focus>>, SurfaceQuery), _>(
					|(focused, surfaces)| {
						let added_surface = surfaces.surface_of(added);
						focused
							.iter()
							.filter(|entity| *entity != added)
							.filter(|entity| {
								surfaces.surface_of(*entity) == added_surface
							})
							.collect::<Vec<_>>()
					},
				);
			for entity in stale {
				world.entity_mut(entity).remove::<Focus>();
			}
		});
	}
}

/// Marker that focuses its element the moment it is added.
///
/// The declarative form of "this element should start focused": its `on_add`
/// inserts [`Focus`] (whose own `on_add` enforces one-focus-per-surface), so
/// markup or a constructor can request initial focus without an observer that
/// re-fires on scene reload. Insert it where the page is built (eg a chat
/// composer's `<input>`).
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
#[component(on_add = Self::on_add)]
pub struct FocusOnAdd;

impl FocusOnAdd {
	/// A [`DeferredWorld`] hook may insert a component (it queues via
	/// `commands()`), exactly as [`Focus::on_add`] does.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		world.commands().entity(cx.entity).insert(Focus);
	}
}

/// Marks a `<form>` whose `<input>`/`<textarea>` values clear when it submits,
/// so the next entry starts empty.
///
/// Generic form behavior, sat beside [`Focus`]: the submitted value is already
/// gathered into the `Submit` event before the clearing observer fires, so
/// clearing never drops it. Knows nothing of any specific composer.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
pub struct ClearOnSubmit;

/// Observer: on [`Submit`], clear every `<input>`/`<textarea>` [`Value`] inside
/// the submitted form's own subtree.
///
/// Gated solely on the submitted form carrying [`ClearOnSubmit`] (no ancestor
/// walk). The target entities are collected before the `&mut Value` loop because
/// the descendant walk borrows immutably while the write borrows mutably. The
/// descendant walk reads `Children`/`Element` rather than
/// [`ElementQuery`](crate::prelude::ElementQuery), so the read and the
/// `&mut Value` write stay disjoint queries.
#[cfg(feature = "template")]
fn clear_on_submit(
	ev: On<Submit>,
	forms: Query<(), With<ClearOnSubmit>>,
	children: Query<&Children>,
	elements: Query<&Element>,
	mut values: Query<&mut Value>,
) {
	if !forms.contains(ev.form) {
		return;
	}
	let inputs = children
		.iter_descendants_inclusive::<Children>(ev.form)
		.filter(|entity| {
			elements
				.get(*entity)
				.map(|element| matches!(element.tag(), "input" | "textarea"))
				.unwrap_or(false)
		})
		.collect::<Vec<_>>();
	for input in inputs {
		if let Ok(mut value) = values.get_mut(input) {
			*value = Value::str("");
		}
	}
}

/// Marks an element that can receive keyboard [`Focus`].
///
/// Auto-inferred from the tag (`<input>`/`<button>`/`<a>`/`<textarea>`) by
/// [`infer_focusable`], or inserted directly. [`tab_focus`] cycles focus through
/// focusables in document (tree) order.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
pub struct Focusable;

/// Tags that are focusable by default, mirroring the browser's sequential focus
/// navigation order.
///
/// `summary` is in the list for the same reason a browser puts it there: it is
/// the control half of a `<details>`, so a disclosure a mouse can open is one
/// Tab can reach. The keyboard half's Enter activation synthesizes the
/// `PointerUp` its toggle listens for, so Enter opens it with nothing else.
pub(super) const FOCUSABLE_TAGS: &[&str] =
	&["input", "button", "a", "textarea", "select", "summary"];

/// Registers the focus model: focusable inference, click-to-focus and the
/// `:focus` state sync, plus under `keyboard` the key routing of the
/// keyboard half (Tab traversal, Enter activation and text entry into the
/// focused [`Value`]).
///
/// Backend-agnostic: whoever assembles the app adds this alongside the renderer
/// plugins. The input systems run in `Update`, after each backend's input
/// collection has buffered its keys and [`PointerDown`]s.
#[derive(Default)]
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Focus>()
			.register_type::<FocusOnAdd>()
			.register_type::<Focusable>()
			.register_type::<ClearOnSubmit>()
			.add_observer(infer_focusable)
			.add_observer(focus_on_click)
			.add_systems(PostUpdate, sync_focus_state);
		// `clear_on_submit` reads the `Submit` event, which lives in the
		// `template`-gated form widgets.
		#[cfg(feature = "template")]
		app.add_observer(clear_on_submit);
		// the message registration is idempotent, so the key systems validate
		// even when no input plugin is composed in
		#[cfg(feature = "keyboard")]
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_systems(
				Update,
				(
					super::keyboard::tab_focus,
					super::keyboard::activate_focused_on_enter,
					super::keyboard::write_focus_input,
				),
			);
	}
}

/// Observer: infer [`Focusable`] from a newly-added element's tag.
fn infer_focusable(
	ev: On<Add, Element>,
	elements: Query<&Element>,
	mut commands: Commands,
) {
	if let Ok(element) = elements.get(ev.entity) {
		if FOCUSABLE_TAGS.contains(&element.tag()) {
			commands.entity(ev.entity).insert(Focusable);
		}
	}
}

/// Observer: clicking a [`Focusable`] focuses it (the `on_add` hook clears focus
/// from others). `PointerDown` auto-propagates, so a click on text inside a
/// focusable still focuses the focusable.
fn focus_on_click(
	ev: On<PointerDown>,
	focusables: Query<(), With<Focusable>>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if focusables.contains(target) {
		commands.entity(target).insert(Focus);
	}
}

/// System: keep the focused entity's [`ElementStateMap`] carrying
/// [`Focused`](ElementState::Focused), so `:focus`/`:focus-visible` rules apply,
/// and clear it from unfocused elements.
///
/// Touches a map only when its `Focused` membership actually changes, so the
/// style cascade re-resolves on focus change but not every frame.
fn sync_focus_state(
	focused: Query<Entity, With<Focus>>,
	mut states: Query<(Entity, &mut ElementStateMap)>,
	without_map: Query<Entity, (With<Focus>, Without<ElementStateMap>)>,
	mut commands: Commands,
) {
	// every focused element (one per surface) carries the `Focused` state.
	let focus = focused.iter().collect::<HashSet<_>>();
	for (entity, mut map) in states.iter_mut() {
		let should = focus.contains(&entity);
		if should && !map.contains(&ElementState::Focused) {
			map.insert(ElementState::Focused);
		} else if !should && map.contains(&ElementState::Focused) {
			map.remove(&ElementState::Focused);
		}
	}
	// the focused entity may not have a state map yet; give it one.
	for entity in without_map.iter() {
		commands
			.entity(entity)
			.insert(ElementStateMap::with(ElementState::Focused));
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::RenderSurface;

	/// Builds an [`App`] with the focus model wired up.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(FocusPlugin);
		app
	}

	/// Clones the [`Value`] currently on `entity`.
	fn value_of(app: &App, entity: Entity) -> Value {
		app.world().entity(entity).get::<Value>().unwrap().clone()
	}

	/// Whether `entity` currently holds [`Focus`].
	fn is_focused(app: &App, entity: Entity) -> bool {
		app.world().entity(entity).contains::<Focus>()
	}

	#[beet_core::test]
	fn single_focus_invariant() {
		let mut app = app();
		let first = app.world_mut().spawn(Focus).id();
		let second = app.world_mut().spawn(Focus).id();
		app.update(); // flush the queued on_add command
		app.world().entity(first).contains::<Focus>().xpect_false();
		app.world().entity(second).contains::<Focus>().xpect_true();
	}

	/// Two surfaces keep their own focused field (the per-surface single-focus
	/// invariant).
	#[beet_core::test]
	fn focus_is_per_surface() {
		let mut app = app();
		let window_a = app.world_mut().spawn_empty().id();
		let window_b = app.world_mut().spawn_empty().id();
		let field_a = app.world_mut().spawn((Focusable, Value::str(""))).id();
		let field_b = app.world_mut().spawn((Focusable, Value::str(""))).id();
		app.world_mut()
			.spawn(RenderSurface(window_a))
			.add_child(field_a);
		app.world_mut()
			.spawn(RenderSurface(window_b))
			.add_child(field_b);
		app.world_mut().entity_mut(field_a).insert(Focus);
		app.world_mut().entity_mut(field_b).insert(Focus);
		app.update();
		is_focused(&app, field_a).xpect_true();
		is_focused(&app, field_b).xpect_true();
	}

	/// `<input>`/`<button>`/`<a>` infer [`Focusable`]; a `<span>` does not.
	#[beet_core::test]
	fn infers_focusable_from_tag() {
		let mut app = app();
		let input = app.world_mut().spawn(Element::new("input")).id();
		let span = app.world_mut().spawn(Element::new("span")).id();
		app.update();
		app.world()
			.entity(input)
			.contains::<Focusable>()
			.xpect_true();
		app.world()
			.entity(span)
			.contains::<Focusable>()
			.xpect_false();
	}

	/// Clicking a focusable focuses it.
	#[beet_core::test]
	fn click_focuses() {
		let mut app = app();
		let input = app.world_mut().spawn((Element::new("input"),)).id();
		app.update(); // infer Focusable
		let pointer = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(input)
			.trigger(PointerDown::new(pointer));
		app.update();
		is_focused(&app, input).xpect_true();
	}

	/// The focused entity carries the `Focused` state (for `:focus`), and it
	/// clears when focus moves away.
	#[beet_core::test]
	fn focus_sets_focused_state() {
		let mut app = app();
		let a = app.world_mut().spawn((Focusable, Focus)).id();
		let b = app.world_mut().spawn(Focusable).id();
		app.update();
		app.world()
			.entity(a)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_true();
		// move focus to b; a's Focused state clears, b's sets
		app.world_mut().entity_mut(b).insert(Focus);
		app.update();
		app.world()
			.entity(a)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_false();
		app.world()
			.entity(b)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_true();
	}

	/// A `:focus` rule resolves on the focused element (the `:focus-visible` style
	/// hook), changing its resolved style; clearing focus reverts it.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn focus_visible_style_applies() {
		use crate::prelude::*;
		use crate::style::*;
		let mut app = App::new();
		// RealtimeParsePlugin runs the style cascade (PostParseTree) each frame.
		app.add_plugins((
			MinimalPlugins,
			CharcellPlugin,
			RealtimeParsePlugin,
			FocusPlugin,
		));
		let ring = Color::srgb(0.1, 0.4, 0.9);
		app.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(vec![
				// `input:focus { border-color: <ring> }`, the focus-visible ring.
				Rule::new()
					.with_selector(Selector::AllOf(vec![
						Selector::tag("input"),
						Selector::state(ElementState::Focused),
					]))
					.with_value(common_props::BorderColorProp, ring),
			]);
		let input = app.world_mut().spawn(Element::new("input")).id();
		// settle: resolve the unfocused style (the focus ring rule does not apply)
		app.update();
		app.update();
		let border_top = |app: &App| {
			app.world()
				.get::<BoxStyle>(input)
				.and_then(|box_style| box_style.border_top)
		};
		let unfocused = border_top(&app);
		// focus it: a couple of frames let the Focused state propagate into the
		// cascade, so the `:focus` rule resolves and recolors the border.
		app.world_mut().entity_mut(input).insert(Focus);
		app.update();
		app.update();
		let focused = border_top(&app);
		focused.xpect_eq(Some(ring));
		(focused != unfocused).xpect_true();
	}

	/// [`FocusOnAdd`] focuses its element the moment it is added.
	#[beet_core::test]
	fn focus_on_add_focuses() {
		let mut app = app();
		let entity = app.world_mut().spawn(FocusOnAdd).id();
		// the on_add hook inserts Focus via a command, settled on the next update.
		app.update();
		is_focused(&app, entity).xpect_true();
	}

	/// [`SurfaceQuery`] resolves an element to its surface through an extra
	/// wrapper element between it and the [`RenderSurface`] host: the walk follows
	/// `ChildOf` to the host regardless of depth.
	#[beet_core::test]
	fn surface_resolves_through_wrapper() {
		let mut app = app();
		let window = app.world_mut().spawn_empty().id();
		// host(RenderSurface) > div wrapper > element, so the element is two hops
		// below the surface, not directly on it.
		let element = app.world_mut().spawn(Element::new("input")).id();
		let wrapper = app
			.world_mut()
			.spawn(Element::new("div"))
			.add_child(element)
			.id();
		app.world_mut()
			.spawn(RenderSurface(window))
			.add_child(wrapper);
		app.world_mut()
			.with_state::<SurfaceQuery, _>(|surfaces| {
				surfaces.surface_of(element)
			})
			.xpect_eq(Some(window));
	}

	/// [`ClearOnSubmit`] empties a plain form's `<input>`/`<textarea>` values on
	/// submit, no thread/composer involved, proving the marker is generic.
	#[cfg(feature = "template")]
	#[beet_core::test]
	fn clear_on_submit_empties_plain_form() {
		let mut app = app();
		// a bare form carrying ClearOnSubmit, with a filled input and textarea.
		let input = app
			.world_mut()
			.spawn((Element::new("input"), Value::str("typed")))
			.id();
		let area = app
			.world_mut()
			.spawn((Element::new("textarea"), Value::str("note")))
			.id();
		let form = app
			.world_mut()
			.spawn((Element::new("form"), ClearOnSubmit))
			.add_children(&[input, area])
			.id();
		app.update();

		// submitting the form clears both fields (the value is already gathered
		// into Submit before the observer fires).
		app.world_mut().trigger(Submit {
			form,
			values: Value::Null,
		});
		app.update();
		value_of(&app, input).xpect_eq(Value::str(""));
		value_of(&app, area).xpect_eq(Value::str(""));
	}

	/// [`ClearOnSubmit`] only touches a form that carries it: a submit on a form
	/// without the marker leaves its fields untouched.
	#[cfg(feature = "template")]
	#[beet_core::test]
	fn clear_on_submit_ignores_unmarked_form() {
		let mut app = app();
		let input = app
			.world_mut()
			.spawn((Element::new("input"), Value::str("typed")))
			.id();
		let form = app
			.world_mut()
			.spawn(Element::new("form"))
			.add_child(input)
			.id();
		app.update();
		app.world_mut().trigger(Submit {
			form,
			values: Value::Null,
		});
		app.update();
		value_of(&app, input).xpect_eq(Value::str("typed"));
	}
}
