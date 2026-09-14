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
//!
//! Typing refines it: a character typed on a focused select opens the panel
//! (if it is closed) and narrows its rows to the options whose label contains
//! what was typed, shown as a filter line at the panel's top, with the first
//! matching row focused so Enter chooses it; Backspace widens it again. The
//! browser's type-to-jump, made visible, and what makes a select over hundreds
//! of options (a scene's component picker) usable on a screen of rows.
//!
//! The panel fits where it opens: the `.select-dropdown` rule flips it above
//! the control when it does not fit below (`position-try-fallbacks`, resolved
//! by the layout pass against the nearest scroll port) and caps it to the room
//! on its side, where it scrolls its rows, the focused row kept in view and
//! the filter line pinned at its top.

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

/// A spawned dropdown panel, pointing back at its owning `<select>` and
/// carrying the text its rows are refined by.
#[derive(Debug, Clone, Component)]
pub struct SelectDropdown {
	pub select: Entity,
	/// What has been typed since the panel opened: only options whose label
	/// contains it (case-insensitively) have a row.
	pub filter: String,
}

/// Marks the panel's filter line, showing what its rows are refined by.
#[derive(Debug, Clone, Copy, Component)]
pub struct SelectFilterLine;

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
			.add_systems(Update, (close_on_escape, refine_on_type));
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

/// System: a character typed on a focused `<select>` opens it, refined to the
/// options whose label contains what was typed; Backspace widens the filter
/// again. The first matching row takes focus, the listbox highlight that
/// follows typing, so Enter chooses it and the panel scrolls it into view. A
/// key reaches the select whose surface it came from.
fn refine_on_type(
	mut keys: MessageReader<KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	elements: ElementQuery,
	values: Query<&Value>,
	open: Query<&SelectOpen>,
	mut dropdowns: Query<&mut SelectDropdown>,
	parents: Query<&ChildOf>,
	surfaces: SurfaceQuery,
	mut commands: Commands,
) {
	// the typed edits, grouped by the surface they came from
	let mut edits = HashMap::<Entity, Vec<Option<String>>>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let edit = match &key.logical_key {
			Key::Character(chars) => Some(Some(chars.to_string())),
			Key::Space => Some(Some(" ".to_string())),
			Key::Backspace => Some(None),
			_ => None,
		};
		if let Some(edit) = edit {
			edits.entry(key.window).or_default().push(edit);
		}
	}
	if edits.is_empty() {
		return;
	}
	// the select the focus is on or in: the control itself, or a row of its
	// open panel
	let selects = focused
		.iter()
		.filter_map(|entity| {
			ancestors_inclusive(entity, &parents).find(|entity| {
				elements
					.get(*entity)
					.is_ok_and(|view| view.tag() == "select")
			})
		})
		.collect::<HashSet<_>>();
	for select in selects {
		let edits = edits
			.iter()
			.filter(|(window, _)| surfaces.matches(select, **window))
			.flat_map(|(_, edits)| edits.iter().cloned())
			.collect::<Vec<_>>();
		if edits.is_empty() {
			continue;
		}
		let mut filter = open
			.get(select)
			.ok()
			.and_then(|open| dropdowns.get(open.dropdown).ok())
			.map(|dropdown| dropdown.filter.clone())
			.unwrap_or_default();
		for edit in edits {
			match edit {
				Some(chars) => filter.push_str(&chars),
				None => {
					filter.pop();
				}
			}
		}
		let first_row = match open.get(select) {
			Ok(open) => {
				if let Ok(mut dropdown) = dropdowns.get_mut(open.dropdown) {
					dropdown.filter = filter.clone();
				}
				commands.entity(open.dropdown).despawn_children();
				spawn_rows(
					&mut commands,
					&elements,
					&values,
					select,
					open.dropdown,
					&filter,
				)
			}
			Err(_) => open_select_filtered(
				&mut commands,
				&elements,
				&values,
				select,
				filter,
			),
		};
		// no match leaves an empty panel: focus returns to the control so the
		// next Backspace still reaches it
		commands.entity(first_row.unwrap_or(select)).insert(Focus);
	}
}

/// Spawn the dropdown panel under `select`: one focusable row per `<option>`,
/// the row matching the current selection carrying the `Selected` state.
/// Focus stays on the control, so Tab walks into the rows as it always has.
fn open_select(
	commands: &mut Commands,
	elements: &ElementQuery,
	values: &Query<&Value>,
	select: Entity,
) {
	open_select_filtered(commands, elements, values, select, String::new());
}

/// [`open_select`], refined to the options whose label contains `filter`.
/// Returns the panel's first row, if any option matched.
fn open_select_filtered(
	commands: &mut Commands,
	elements: &ElementQuery,
	values: &Query<&Value>,
	select: Entity,
	filter: String,
) -> Option<Entity> {
	let panel = commands
		.spawn((
			Element::new("div"),
			Classes::new([classes::SELECT_DROPDOWN]),
			SelectDropdown {
				select,
				filter: filter.clone(),
			},
			ChildOf(select),
		))
		.id();
	let first_row =
		spawn_rows(commands, elements, values, select, panel, &filter);
	commands
		.entity(select)
		.insert(SelectOpen { dropdown: panel });
	first_row
}

/// The panel's rows: the filter line when there is a filter, then one
/// focusable row per option whose label contains it, the row matching the
/// current selection carrying the `Selected` state. Returns the first row, if
/// any option matched.
fn spawn_rows(
	commands: &mut Commands,
	elements: &ElementQuery,
	values: &Query<&Value>,
	select: Entity,
	panel: Entity,
	filter: &str,
) -> Option<Entity> {
	if !filter.is_empty() {
		commands.spawn((
			Element::new("div").with_inner_text(&format!("/{filter}")),
			Classes::new([classes::SELECT_FILTER]),
			SelectFilterLine,
			ChildOf(panel),
		));
	}
	let selected = selected_value(elements, values, select);
	let needle = filter.to_lowercase();
	let mut first_row = None;
	for option in select_options(elements, select) {
		let label = option.option_label();
		if !label.to_lowercase().contains(&needle) {
			continue;
		}
		let value = option.option_value();
		let mut row = commands.spawn((
			Element::new("div").with_inner_text(&label),
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
		first_row.get_or_insert(row.id());
	}
	first_row
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
				.map(|option| option.option_value())
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

	/// A host showing `content`, stepped once so it is painted. Material rules
	/// style the control and position the dropdown, exactly as a real app
	/// composes them.
	fn host_showing(content: impl Bundle) -> TestHost {
		let mut host = TestHost::sized(UVec2::new(30, 14));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		host.spawn_content(content);
		host.step();
		host
	}

	/// A host showing a `Select` with two options.
	fn select_host() -> TestHost {
		// wrapped in a block container (and followed by a sibling) so the select
		// takes its content height and the panel must overlay what follows.
		host_showing(rsx! {
			<div>
				<Select name="role">
					<option value="engineer">"Engineer"</option>
					<option value="designer">"Designer"</option>
				</Select>
				<p>"below"</p>
			</div>
		})
	}

	/// Every `<select>` in the host, in spawn order.
	fn select_entities(host: &mut TestHost) -> Vec<Entity> {
		host.app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(host.app.world())
			.filter(|(_, element)| element.tag() == "select")
			.map(|(entity, _)| entity)
			.collect()
	}

	fn select_entity(host: &mut TestHost) -> Entity {
		select_entities(host).remove(0)
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

	/// The row for `value`. By value, never by query index: iteration order is
	/// archetype order, which differs across targets.
	fn row_by_value(host: &mut TestHost, value: &str) -> Entity {
		rows(host)
			.into_iter()
			.find(|(_, row)| row.value == value)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// The entity holding keyboard focus, if any.
	fn focused(host: &mut TestHost) -> Option<Entity> {
		host.app
			.world_mut()
			.query_filtered::<Entity, With<Focus>>()
			.iter(host.app.world())
			.next()
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

	/// A `<select>` that is the only child of a block container keeps its box and
	/// opens a real panel.
	///
	/// The lone-child case is the one a generated form hits: a payload-less enum
	/// variant (`on_missing: null`, a new field's `Any`) puts the variant select
	/// alone in its holder, where an inline formatting context used to flatten
	/// the control's border away and splice the open dropdown's rows into the
	/// same text run — `▾AnyAnyNullBoolI64…` on one line.
	#[beet_core::test]
	fn lone_select_keeps_its_box_and_panel() {
		let mut host = host_showing(rsx! {
			<div>
				<Select name="role">
					<option value="alpha">"Alpha"</option>
					<option value="beta">"Beta"</option>
				</Select>
			</div>
		});
		host.frame_plain().xpect_contains("┌");
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.frame_plain().xnot().xpect_contains("AlphaBeta");
	}

	/// The same shape a `DynamicForm` generates: a payload-less enum variant puts
	/// the variant select alone in its holder, and it must still open a panel.
	#[beet_core::test]
	fn generated_variant_select_opens_a_panel() {
		let mut host = TestHost::sized(UVec2::new(40, 20));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		host.spawn_content(rsx! {
			<DynamicForm
				schema={ValueSchema::Enum(EnumSchema {
					// payload-carrying, so the form generates a `VariantSelect`;
					// the *chosen* variant `Alpha` is a unit one, so the select is
					// the holder's only child
					variants: vec![
						VariantSchema { name: "Alpha".into(), payload: None },
						VariantSchema {
							name: "Beta".into(),
							payload: Some(ValueSchema::Bool(default())),
						},
					],
					..default()
				})}
				field={FieldRef::new("choice")}/>
		});
		for _ in 0..4 {
			host.step();
		}
		host.frame_plain().xpect_contains("┌");
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.frame_plain().xnot().xpect_contains("AlphaBeta");
	}

	/// A lone text control keeps its box once it holds a value.
	///
	/// The same mechanism read from the other end: a control counts as
	/// inline-level only when it has something to show, so before this fix an
	/// `<input>` alone in a `<div>` was a proper box while empty and lost its
	/// border the moment you typed into it.
	#[beet_core::test]
	fn lone_text_field_keeps_its_box_when_filled() {
		let host = host_showing(rsx! {
			<div><TextField {Value::new("typed")}/></div>
		});
		host.frame_plain()
			.as_str()
			.xpect_contains("typed")
			.xpect_contains("┌");
	}

	/// An authored `<select>` inside a `<label>` opens its panel too: the label
	/// is a flex column (the shipped rule, so a key sits above its control), and
	/// an absolutely positioned dropdown must still leave that flow rather than
	/// being laid out as another flex item.
	#[beet_core::test]
	fn labelled_select_opens_a_panel() {
		let mut host = TestHost::sized(UVec2::new(40, 20));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		host.spawn_content(rsx! {
			<label>"role"
				<Select name="role">
					<option value="alpha">"Alpha"</option>
					<option value="beta">"Beta"</option>
				</Select>
			</label>
		});
		host.step();
		host.step();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.frame_plain().xnot().xpect_contains("AlphaBeta");
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
		let designer = row_by_value(&mut host, "designer");
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
		let second = row_by_value(&mut host, "designer");
		focused(&mut host).xpect_eq(Some(second));
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

	/// Typing on a focused select opens it refined to the matching options,
	/// shown under a filter line with the first match focused; Backspace widens
	/// the rows again, and the select's own value is never typed into.
	#[beet_core::test]
	fn typing_opens_and_refines() {
		let mut host = select_host();
		let surface = host.host;
		let select = select_entity(&mut host);
		host.app
			.world_mut()
			.entity_mut(select)
			.insert((Focus, RenderSurface(surface)));
		host.step();
		host.send_input(b"des");
		host.step();
		host.step();
		dropdown(&mut host).xpect_some();
		rows(&mut host)
			.into_iter()
			.map(|(_, row)| row.value)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["designer".to_string()]);
		host.frame_plain().xpect_contains("/des");
		// the match is highlighted, so Enter would choose it
		let (designer, _) = rows(&mut host).remove(0);
		focused(&mut host).xpect_eq(Some(designer));
		host.app
			.world()
			.get::<Value>(select)
			.unwrap()
			.clone()
			.xpect_eq(Value::str(""));

		// a filter matching nothing empties the panel and hands focus back to
		// the control, so backspacing the filter away still widens the panel
		// to every option
		host.send_input(b"x");
		host.step();
		host.step();
		rows(&mut host).len().xpect_eq(0);
		focused(&mut host).xpect_eq(Some(select));
		host.send_input(b"\x7f\x7f\x7f\x7f");
		host.step();
		host.step();
		rows(&mut host).len().xpect_eq(2);
		host.frame_plain().xnot().xpect_contains("/");

		// choosing from the refined rows works as it always did
		host.send_input(b"eng");
		host.step();
		host.step();
		let (engineer, _) = rows(&mut host).remove(0);
		activate(&mut host, engineer);
		host.step();
		host.app
			.world()
			.get::<Value>(select)
			.unwrap()
			.clone()
			.xpect_eq(Value::str("engineer"));
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

	/// A select on the last rows of a short viewport opens upward, every option
	/// visible: the panel's `top: 100%` would run off the screen, so the layout
	/// flips it to `bottom: 100%` and it hangs from the control's top edge.
	#[beet_core::test]
	fn opens_upward_when_below_does_not_fit() {
		let mut host = TestHost::sized(UVec2::new(30, 10));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		// six one-row fillers push the three-row control to the bottom rows
		host.spawn_content(rsx! {
			<div>
				{(0..6).map(|i| rsx! { <div>{format!("filler {i}")}</div> }).collect::<Vec<_>>()}
				<Select name="role">
					<option value="engineer">"Engineer"</option>
					<option value="designer">"Designer"</option>
				</Select>
			</div>
		});
		host.step();
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.step();
		let frame = host.frame_plain();
		frame.as_str().xpect_snapshot();
		// both rows paint, above the control's caret row
		let row_of = |needle: &str| {
			frame
				.lines()
				.position(|line| line.contains(needle))
				.unwrap()
		};
		(row_of("Designer") < row_of("▾")).xpect_true();
	}

	/// Thirty options on a twelve-row host: the panel is capped to the room
	/// below the control and scrolls its rows, Tab past the visible rows
	/// scrolls the focused one into view, and a typed filter keeps its line
	/// pinned at the panel's top while the rows scroll beneath it.
	#[beet_core::test]
	fn tall_dropdown_is_capped_and_scrolls() {
		let mut host = TestHost::sized(UVec2::new(30, 12));
		host.app
			.add_plugins(crate::style::material::MaterialStylePlugin::default());
		host.spawn_content(rsx! {
			<div>
				<Select name="n">
					{(1..=30)
						.map(|i| rsx! { <option value=i.to_string()>{format!("Option {i}")}</option> })
						.collect::<Vec<_>>()}
				</Select>
			</div>
		});
		host.step();
		let surface = host.host;
		let select = select_entity(&mut host);
		host.app
			.world_mut()
			.entity_mut(select)
			.insert((Focus, RenderSurface(surface)));
		host.step();
		host.send_input(b"\r");
		host.step();
		host.step();
		// capped: the panel ends at the viewport, the tail rows are not painted,
		// and the panel reserves a scrollbar for them
		let frame = host.frame_plain();
		frame.as_str().xpect_snapshot();
		frame
			.as_str()
			.xpect_contains("Option 1")
			.xnot()
			.xpect_contains("Option 30");
		frame.as_str().xpect_contains("█");
		// Tab to the twentieth row scrolls it into view
		for _ in 0..20 {
			host.send_input(b"\t");
			host.step();
		}
		let twentieth = row_by_value(&mut host, "20");
		focused(&mut host).xpect_eq(Some(twentieth));
		// the minimum scroll: the focused row sits one row of context above the
		// panel's bottom edge
		host.frame_plain()
			.as_str()
			.xpect_contains("Option 20")
			.xpect_contains("Option 21")
			.xnot()
			.xpect_contains("Option 22");
		// a filter narrows the rows to the twelve containing "2", the first of
		// them focused; Tab down them scrolls the panel with the filter line
		// still on its first row
		host.send_input(b"2");
		host.step();
		host.step();
		let frame = host.frame_plain();
		frame.as_str().xpect_contains("/2");
		rows(&mut host).len().xpect_eq(12);
		for _ in 0..8 {
			host.send_input(b"\t");
			host.step();
		}
		// the sticky line pins against the offset the focus scroll wrote after
		// this frame's layout, so it settles a frame later
		host.step();
		let frame = host.frame_plain();
		frame.as_str().xpect_snapshot();
		let row_of = |needle: &str| {
			frame
				.lines()
				.position(|line| line.contains(needle))
				.unwrap()
		};
		(row_of("/2") < row_of("Option 2")).xpect_true();
		frame.as_str().xpect_contains("Option 25");
	}

	/// The panel overlays a following `<select>` too, not just in-flow content:
	/// a select is `position: relative` with no `z-index`, which forms no
	/// stacking context, so the panel's `z-index` sorts against that sibling
	/// rather than being trapped in the open select.
	///
	/// Regression: the second select overdrew the first one's open panel.
	#[beet_core::test]
	fn open_dropdown_overlays_a_following_select() {
		let mut host = host_showing(rsx! {
			<div>
				<Select name="role">
					<option value="engineer">"Engineer"</option>
					<option value="designer">"Designer"</option>
				</Select>
				<Select name="team">
					<option value="alpha">"Alpha"</option>
					<option value="beta">"Beta"</option>
				</Select>
				<p>"below"</p>
			</div>
		});
		let select = select_entity(&mut host);
		activate(&mut host, select);
		host.step();
		// the panel covers the next control's label, so only its box edges remain
		host.frame_plain()
			.xpect_contains("Designer")
			.xnot()
			.xpect_contains("Alpha");
	}

	/// Choosing an option whose *value* is wider than the control (a component
	/// picker's type path) keeps the control at one row: its marker label is
	/// what is measured, in layout as in measure, never the raw value.
	///
	/// Regression: `resolve_height` read the value before the marker, so the
	/// closed control reserved the rows the wrapped type path would take.
	#[beet_core::test]
	fn choosing_a_wide_value_keeps_the_row() {
		let mut host = host_showing(rsx! {
			<div>
				<Select name="component">
					<optgroup label="Components">
						<option value="bevy_ecs::name::Name">"Name"</option>
						<option value="bevy_ecs::hierarchy::ChildOf">"ChildOf"</option>
					</optgroup>
				</Select>
			</div>
		});
		let select = select_entity(&mut host);
		activate(&mut host, select);
		let child_of = row_by_value(&mut host, "bevy_ecs::hierarchy::ChildOf");
		activate(&mut host, child_of);
		host.step();
		host.step();
		host.frame_plain().trim_lines().xpect_eq(
			"┌───────────────────┐\n│  ChildOf ▾        │\n└───────────────────┘",
		);
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
