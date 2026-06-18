//! Native `<select>` dropdown interaction for the live TUI.
//!
//! The closed control renders its selected option's label plus a caret as a
//! [`Marker`] (its `<option>` children are `display: none` on the terminal,
//! like a browser's closed select). Activating the select — click, or Enter
//! while focused — spawns a dropdown panel: an absolutely positioned element
//! anchored below the control, overlaying subsequent content. Each option
//! becomes a focusable row, so Tab cycles rows and Enter or a click chooses
//! one, writing the select's [`Value`] (the form submission value). Escape or
//! a click away dismisses the panel.

use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;

/// Marks an open `<select>`, pointing at its spawned dropdown panel.
#[derive(Debug, Clone, Copy, Component)]
pub struct SelectOpen {
	/// The spawned `.select-dropdown` panel.
	pub dropdown: Entity,
}

/// A spawned dropdown panel, pointing back at its owning `<select>`.
#[derive(Debug, Clone, Copy, Component)]
pub struct SelectDropdown {
	pub select: Entity,
}

/// A dropdown row for one `<option>`, carrying the submission value it writes
/// into its select's [`Value`] when chosen.
#[derive(Debug, Clone, Component)]
pub struct SelectOptionRow {
	pub select: Entity,
	pub value: String,
}

/// Registers the dropdown interaction: toggle on select activation, choose on
/// row activation, dismiss on Escape or a press outside the open select.
#[derive(Default)]
pub struct SelectPlugin;

impl Plugin for SelectPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(toggle_select_on_click)
			.add_observer(choose_option_on_click)
			.add_observer(close_on_press_away)
			.add_systems(Update, close_on_escape);
	}
}

/// Observer: activating a `<select>` (click, or Enter while focused via the
/// keyboard activation path) toggles its dropdown panel.
fn toggle_select_on_click(
	ev: On<PointerUp>,
	elements: ElementQuery,
	values: Query<&Value>,
	open: Query<&SelectOpen>,
	dropdowns: Query<(), With<SelectDropdown>>,
	parents: Query<&ChildOf>,
	mut commands: Commands,
) {
	// the event bubbles; act only at the `<select>` step of the propagation.
	let select = ev.event_target();
	if !elements
		.get(select)
		.is_ok_and(|view| view.tag() == "select")
	{
		return;
	}
	// a click originating inside the panel (a row) bubbles through the select;
	// the row handler owns it.
	if inside_dropdown(ev.original_event_target(), &dropdowns, &parents) {
		return;
	}
	match open.get(select) {
		Ok(open) => close_select(&mut commands, select, open.dropdown, true),
		Err(_) => open_select(&mut commands, &elements, &values, select),
	}
}

/// Observer: choosing a row writes its value into the select and closes.
fn choose_option_on_click(
	ev: On<PointerUp>,
	rows: Query<&SelectOptionRow>,
	open: Query<&SelectOpen>,
	mut values: Query<&mut Value>,
	mut commands: Commands,
) {
	let Ok(row) = rows.get(ev.event_target()) else {
		return;
	};
	match values.get_mut(row.select) {
		Ok(mut value) => {
			value.set_if_neq(Value::str(row.value.as_str()));
		}
		// no Value yet (FormPlugin absent): attach the selection directly
		Err(_) => {
			commands
				.entity(row.select)
				.insert(Value::str(row.value.as_str()));
		}
	}
	if let Ok(open) = open.get(row.select) {
		close_select(&mut commands, row.select, open.dropdown, true);
	}
}

/// Observer: a press outside an open select (and its panel) dismisses it, like
/// a browser's light-dismiss.
fn close_on_press_away(
	ev: On<PointerDown>,
	open: Query<(Entity, &SelectOpen)>,
	parents: Query<&ChildOf>,
	mut commands: Commands,
) {
	// the event bubbles; act once, at the original target.
	if ev.event_target() != ev.original_event_target() {
		return;
	}
	for (select, select_open) in open.iter() {
		let inside = ancestors_inclusive(ev.original_event_target(), &parents)
			.any(|entity| entity == select);
		if !inside {
			close_select(&mut commands, select, select_open.dropdown, false);
		}
	}
}

/// ECS system: Escape dismisses every open select, refocusing the control.
fn close_on_escape(
	mut keys: MessageReader<KeyboardInput>,
	open: Query<(Entity, &SelectOpen)>,
	mut commands: Commands,
) {
	let escaped = keys.read().any(|key| {
		key.state == ButtonState::Pressed && key.logical_key == Key::Escape
	});
	if !escaped {
		return;
	}
	for (select, select_open) in open.iter() {
		close_select(&mut commands, select, select_open.dropdown, true);
	}
}

/// Spawn the dropdown panel under `select`: one focusable row per `<option>`,
/// the row matching the current selection carrying the `Selected` state.
fn open_select(
	commands: &mut Commands,
	elements: &ElementQuery,
	values: &Query<&Value>,
	select: Entity,
) {
	let selected = selected_value(elements, values, select);
	let panel = commands
		.spawn((
			Element::new("div"),
			Classes::new([classes::SELECT_DROPDOWN]),
			SelectDropdown { select },
			ChildOf(select),
		))
		.id();
	for option in select_options(elements, select) {
		let value = option_value(&option);
		let mut row = commands.spawn((
			Element::new("div").with_inner_text(&option_label(&option)),
			Classes::new([classes::SELECT_OPTION]),
			SelectOptionRow {
				select,
				value: value.clone(),
			},
			Focusable,
			ChildOf(panel),
		));
		if Some(value) == selected {
			row.insert(ElementStateMap::with(ElementState::Selected));
		}
	}
	commands
		.entity(select)
		.insert(SelectOpen { dropdown: panel });
}

/// Despawn the panel; `refocus` returns keyboard focus to the select (chosen
/// or Escape-dismissed), while a press elsewhere leaves focus where it landed.
fn close_select(
	commands: &mut Commands,
	select: Entity,
	dropdown: Entity,
	refocus: bool,
) {
	commands.entity(dropdown).despawn();
	commands.entity(select).remove::<SelectOpen>();
	if refocus {
		commands.entity(select).insert(Focus);
	}
}

/// The select's current submission value: its edited [`Value`], else its first
/// option's value (the browser's default selection), else `None` (no options).
fn selected_value(
	elements: &ElementQuery,
	values: &Query<&Value>,
	select: Entity,
) -> Option<String> {
	values
		.get(select)
		.ok()
		.and_then(|value| value.as_str().ok())
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string())
		.or_else(|| {
			select_options(elements, select)
				.next()
				.map(|option| option_value(&option))
		})
}

/// The `<option>` views under `select`, in document order.
fn select_options<'a>(
	elements: &'a ElementQuery,
	select: Entity,
) -> impl Iterator<Item = ElementView<'a>> {
	elements
		.iter_descendants_inclusive(select)
		.filter(|view| view.tag() == "option")
}

/// Whether `entity` sits inside a spawned dropdown panel (inclusive).
fn inside_dropdown(
	entity: Entity,
	dropdowns: &Query<(), With<SelectDropdown>>,
	parents: &Query<&ChildOf>,
) -> bool {
	ancestors_inclusive(entity, parents)
		.any(|entity| dropdowns.contains(entity))
}

/// `entity` then its ancestors, root-ward.
fn ancestors_inclusive<'a>(
	entity: Entity,
	parents: &'a Query<&ChildOf>,
) -> impl Iterator<Item = Entity> + 'a {
	[entity].into_iter().chain(parents.iter_ancestors(entity))
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::render::charcell::test_host::TestHost;
	use bevy::math::UVec2;

	/// A host showing a `Select` with two options, stepped once so the closed
	/// control is painted. Material rules style the control and position the
	/// dropdown, exactly as a real app composes them.
	fn select_host() -> TestHost {
		let mut host = TestHost::sized(UVec2::new(30, 14));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		// wrapped in a block container (and followed by a sibling) so the select
		// takes its content height and the panel must overlay what follows.
		host.spawn_content(rsx! {
			<div>
				<Select name="role">
					<option value="engineer">"Engineer"</option>
					<option value="designer">"Designer"</option>
				</Select>
				<p>"below"</p>
			</div>
		});
		host.step();
		host
	}

	fn select_entity(host: &mut TestHost) -> Entity {
		host.app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(host.app.world())
			.find(|(_, element)| element.tag() == "select")
			.map(|(entity, _)| entity)
			.unwrap()
	}

	fn dropdown(host: &mut TestHost) -> Option<Entity> {
		host.app
			.world_mut()
			.query_filtered::<Entity, With<SelectDropdown>>()
			.iter(host.app.world())
			.next()
	}

	fn rows(host: &mut TestHost) -> Vec<(Entity, SelectOptionRow)> {
		host.app
			.world_mut()
			.query::<(Entity, &SelectOptionRow)>()
			.iter(host.app.world())
			.map(|(entity, row)| (entity, row.clone()))
			.collect()
	}

	/// Activate `entity` the way the hit-test/keyboard path does.
	fn activate(host: &mut TestHost, entity: Entity) {
		let pointer = host.app.world_mut().spawn_empty().id();
		host.app
			.world_mut()
			.entity_mut(entity)
			.trigger(PointerDown::new(pointer));
		host.app
			.world_mut()
			.entity_mut(entity)
			.trigger(PointerUp::new(pointer));
		host.step();
	}

	/// The closed control renders its default (first) option label plus the
	/// dropdown caret, with no option rows in flow.
	#[beet_core::test]
	fn closed_select_shows_selected_label() {
		let host = select_host();
		let frame = host.frame_plain();
		frame.as_str().xpect_contains("Engineer ▾");
		frame.xnot().xpect_contains("Designer");
	}

	/// Activating the select opens the panel with one focusable row per option;
	/// activating it again closes it.
	#[beet_core::test]
	fn activation_toggles_dropdown() {
		let mut host = select_host();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		dropdown(&mut host).xpect_some();
		rows(&mut host).len().xpect_eq(2);
		// both option labels are now painted (the panel overlays the page)
		host.frame_plain().xpect_contains("Designer");
		activate(&mut host, select);
		dropdown(&mut host).xpect_none();
	}

	/// Choosing a row writes the select's Value, closes the panel, refocuses
	/// the select, and the closed control re-renders the new label.
	#[beet_core::test]
	fn choosing_a_row_selects_and_closes() {
		let mut host = select_host();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		let designer = rows(&mut host)
			.into_iter()
			.find(|(_, row)| row.value == "designer")
			.map(|(entity, _)| entity)
			.unwrap();
		activate(&mut host, designer);
		host.step();
		dropdown(&mut host).xpect_none();
		host.app
			.world()
			.get::<Value>(select)
			.unwrap()
			.clone()
			.xpect_eq(Value::str("designer"));
		host.app
			.world()
			.entity(select)
			.contains::<Focus>()
			.xpect_true();
		host.step();
		host.frame_plain().xpect_contains("Designer ▾");
	}

	/// The full keyboard path: Enter opens, Tab walks to a row, Enter chooses it.
	#[beet_core::test]
	fn keyboard_opens_tabs_and_chooses() {
		let mut host = select_host();
		let surface = host.host;
		let select = select_entity(&mut host);
		// scope the select (and its `ChildOf`-nested dropdown rows) to the host
		// surface so the per-surface keyboard path delivers Enter/Tab to it.
		host.app
			.world_mut()
			.entity_mut(select)
			.insert((Focus, RenderSurface(surface)));
		host.step();
		// Enter activates the focused select, opening the panel
		host.send_input(b"\r");
		host.step();
		dropdown(&mut host).xpect_some();
		// Tab moves focus to the first row, Tab again to the second
		host.send_input(b"\t");
		host.step();
		host.send_input(b"\t");
		host.step();
		let focused = host
			.app
			.world_mut()
			.query_filtered::<Entity, With<Focus>>()
			.single(host.app.world())
			.unwrap();
		let second = rows(&mut host)
			.into_iter()
			.find(|(_, row)| row.value == "designer")
			.map(|(entity, _)| entity)
			.unwrap();
		focused.xpect_eq(second);
		// Enter chooses the focused row
		host.send_input(b"\r");
		host.step();
		host.step();
		dropdown(&mut host).xpect_none();
		host.app
			.world()
			.get::<Value>(select)
			.unwrap()
			.clone()
			.xpect_eq(Value::str("designer"));
	}

	/// Escape dismisses the open panel without changing the value.
	#[beet_core::test]
	fn escape_closes_without_selecting() {
		let mut host = select_host();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		dropdown(&mut host).xpect_some();
		host.send_input(b"\x1b");
		host.step();
		host.step();
		dropdown(&mut host).xpect_none();
		host.app
			.world()
			.get::<Value>(select)
			.unwrap()
			.clone()
			.xpect_eq(Value::str(""));
	}

	/// The open dropdown overlays the content below the select.
	#[beet_core::test]
	fn open_dropdown_overlays_following_content() {
		let mut host = select_host();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.step();
		host.frame_plain().xpect_snapshot();
	}

	/// A press outside the select and its panel dismisses the panel.
	#[beet_core::test]
	fn press_away_closes() {
		let mut host = select_host();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		dropdown(&mut host).xpect_some();
		let outside = host.app.world_mut().spawn_empty().id();
		let pointer = host.app.world_mut().spawn_empty().id();
		host.app
			.world_mut()
			.entity_mut(outside)
			.trigger(PointerDown::new(pointer));
		host.step();
		dropdown(&mut host).xpect_none();
	}
}
