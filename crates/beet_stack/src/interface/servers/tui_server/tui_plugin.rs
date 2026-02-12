//! The [`TuiPlugin`] wires up [`bevy_ratatui`] for terminal rendering,
//! manages TUI state, and registers default tool input widgets.
use beet_core::prelude::*;
use bevy_ratatui::RatatuiPlugins;
use ratatui::Frame;
use ratatui::layout::Rect as TuiRect;
use ratatui::style::Color as TuiColor;
use ratatui::style::Style as TuiStyle;
use ratatui::text::Span;
use ratatui::widgets::ListState;
use std::any::TypeId;
use std::sync::Arc;

/// Minimum terminal width (columns) to auto-show the card tree panel.
pub(super) const TREE_AUTO_SHOW_WIDTH: u16 = 80;
/// Width of the tree panel in columns.
pub(super) const TREE_PANEL_WIDTH: u16 = 24;

/// A rendering function for a tool input widget.
///
/// Parameters: `(frame, area, current_input_value, is_focused)`
pub type TuiToolRenderer =
	Arc<dyn Fn(&mut Frame, TuiRect, &str, bool) + Send + Sync>;

/// Maps tool input [`TypeId`] to a [`TuiToolRenderer`] that draws
/// an appropriate input widget for that type.
#[derive(Resource, Clone)]
pub struct TuiToolMap(pub HashMap<TypeId, TuiToolRenderer>);

impl Default for TuiToolMap {
	fn default() -> Self {
		let mut map = HashMap::default();

		// `()` input: render as a simple "[Run]" button
		map.insert(
			TypeId::of::<()>(),
			Arc::new(
				|frame: &mut Frame,
				 area: TuiRect,
				 _value: &str,
				 focused: bool| {
					let style = if focused {
						TuiStyle::default().fg(TuiColor::Yellow).bold()
					} else {
						TuiStyle::default().fg(TuiColor::DarkGray)
					};
					let label = if focused { "▸ [Run]" } else { "  [Run]" };
					frame.render_widget(Span::styled(label, style), area);
				},
			) as TuiToolRenderer,
		);

		// String input: render a text field
		map.insert(
			TypeId::of::<String>(),
			Arc::new(
				|frame: &mut Frame,
				 area: TuiRect,
				 value: &str,
				 focused: bool| {
					let style = if focused {
						TuiStyle::default().fg(TuiColor::Yellow)
					} else {
						TuiStyle::default().fg(TuiColor::White)
					};
					let display = format!("[{}]", value);
					frame.render_widget(Span::styled(display, style), area);
				},
			) as TuiToolRenderer,
		);

		// Numeric types: render a number field
		let number_renderer: TuiToolRenderer = Arc::new(
			|frame: &mut Frame, area: TuiRect, value: &str, focused: bool| {
				let style = if focused {
					TuiStyle::default().fg(TuiColor::Cyan)
				} else {
					TuiStyle::default().fg(TuiColor::White)
				};
				let display = format!("[{}]", value);
				frame.render_widget(Span::styled(display, style), area);
			},
		);
		map.insert(TypeId::of::<i32>(), number_renderer.clone());
		map.insert(TypeId::of::<i64>(), number_renderer.clone());
		map.insert(TypeId::of::<u32>(), number_renderer.clone());
		map.insert(TypeId::of::<u64>(), number_renderer.clone());
		map.insert(TypeId::of::<f32>(), number_renderer.clone());
		map.insert(TypeId::of::<f64>(), number_renderer);

		Self(map)
	}
}

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiFocus {
	/// The command palette at the bottom.
	Command,
	/// The card tree sidebar.
	Tree,
	/// The tools list in the main panel.
	Tools,
}

/// Mutable TUI state driving the layout and rendering.
#[derive(Resource)]
pub struct TuiState {
	/// Current text in the command palette.
	pub command_input: String,
	/// Last response body text displayed in the main panel.
	pub response_text: String,
	/// Whether the card tree sidebar is visible.
	pub tree_visible: bool,
	/// Selection state for the tree list widget.
	pub tree_state: ListState,
	/// Which panel has focus.
	pub focus: TuiFocus,
	/// Cached flat list of card paths for the tree panel.
	pub tree_entries: Vec<TreeEntry>,
	/// Index of the currently selected tool in the tools list.
	pub tool_index: usize,
	/// Per-tool input strings keyed by tool path.
	pub tool_inputs: HashMap<String, String>,
	/// Command sender, fed into the async dispatch loop.
	pub command_tx: beet_core::exports::async_channel::Sender<String>,
	/// Response receiver, polled each frame for completed responses.
	pub response_rx: beet_core::exports::async_channel::Receiver<String>,
}

/// An entry in the card tree panel.
#[derive(Debug, Clone)]
pub struct TreeEntry {
	/// Display label.
	pub label: String,
	/// Route path segments for navigation.
	pub path: String,
	/// Nesting depth for indentation.
	pub depth: usize,
}

/// Bevy plugin that sets up [`bevy_ratatui`], inserts TUI resources,
/// and registers the input/draw systems.
///
/// Add this plugin alongside [`StackPlugin`] when building a TUI app.
/// All boilerplate for the terminal lifecycle is handled here.
pub struct TuiPlugin;

impl Plugin for TuiPlugin {
	fn build(&self, app: &mut App) {
		let (cmd_tx, cmd_rx) =
			beet_core::exports::async_channel::unbounded::<String>();
		let (res_tx, res_rx) =
			beet_core::exports::async_channel::unbounded::<String>();

		app.add_plugins(RatatuiPlugins::default())
			.insert_resource(TuiToolMap::default())
			.insert_resource(TuiState {
				command_input: String::new(),
				response_text: String::new(),
				tree_visible: true,
				tree_state: ListState::default(),
				focus: TuiFocus::Command,
				tree_entries: Vec::new(),
				tool_index: 0,
				tool_inputs: HashMap::default(),
				command_tx: cmd_tx,
				response_rx: res_rx,
			})
			.insert_resource(TuiCommandChannel {
				command_rx: cmd_rx,
				response_tx: res_tx,
			})
			.add_systems(PreUpdate, super::tui_input::tui_input_system)
			.add_systems(
				Update,
				(
					super::tui_draw::tui_poll_responses,
					super::tui_draw::tui_sync_tree,
					super::tui_draw::tui_draw_system,
				)
					.chain(),
			);
	}
}

/// Channels shared between the TUI systems and the async dispatch loop
/// running on the [`tui_server`] entity.
#[derive(Resource, Clone)]
pub struct TuiCommandChannel {
	/// Receives commands submitted from the TUI input system.
	pub command_rx: beet_core::exports::async_channel::Receiver<String>,
	/// Sends response bodies back to the TUI for display.
	pub response_tx: beet_core::exports::async_channel::Sender<String>,
}

/// Marker component placed on the entity spawned by [`tui_server`].
#[derive(Component)]
pub struct TuiServer;
