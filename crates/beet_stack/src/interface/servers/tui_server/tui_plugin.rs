//! The [`TuiPlugin`] wires up [`bevy_ratatui`] for terminal rendering,
//! manages TUI state, and registers default tool input widgets.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use bevy_ratatui::RatatuiPlugins;
use bevy_ratatui::event::KeyMessage;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect as TuiRect;
use ratatui::style::Color as TuiColor;
use ratatui::style::Style as TuiStyle;

use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Wrap;
use std::any::TypeId;
use std::sync::Arc;

/// Minimum terminal width (columns) to auto-show the card tree panel.
const TREE_AUTO_SHOW_WIDTH: u16 = 80;
/// Width of the tree panel in columns.
const TREE_PANEL_WIDTH: u16 = 24;

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
			.add_systems(PreUpdate, tui_input_system)
			.add_systems(
				Update,
				(tui_poll_responses, tui_sync_tree, tui_draw_system).chain(),
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

// ── Input ────────────────────────────────────────────────────────────

fn tui_input_system(
	mut messages: MessageReader<KeyMessage>,
	mut exit: MessageWriter<AppExit>,
	mut state: ResMut<TuiState>,
) {
	for message in messages.read() {
		// Ctrl-C always quits
		if message.modifiers.contains(KeyModifiers::CONTROL)
			&& message.code == KeyCode::Char('c')
		{
			exit.write(AppExit::Success);
			return;
		}

		match state.focus {
			TuiFocus::Command => handle_command_input(&mut state, message.code),
			TuiFocus::Tree => handle_tree_input(&mut state, message.code),
			TuiFocus::Tools => handle_tools_input(&mut state, message.code),
		}

		// Global toggles (only when not typing in command palette)
		if state.focus != TuiFocus::Command {
			match message.code {
				KeyCode::Char('q') | KeyCode::Esc => {
					exit.write(AppExit::Success);
					return;
				}
				KeyCode::Char('t') => {
					state.tree_visible = !state.tree_visible;
				}
				_ => {}
			}
		}
	}
}

fn handle_command_input(state: &mut TuiState, code: KeyCode) {
	match code {
		KeyCode::Char(ch) => {
			state.command_input.push(ch);
		}
		KeyCode::Backspace => {
			state.command_input.pop();
		}
		KeyCode::Enter => {
			let command = std::mem::take(&mut state.command_input);
			let trimmed = command.trim().to_string();
			if !trimmed.is_empty() {
				if trimmed == "exit" || trimmed == "quit" {
					// handled next frame via AppExit
					state.command_input = "quit".into();
					return;
				}
				state.command_tx.send_blocking(trimmed).ok();
				state.response_text = "Running...".into();
			}
		}
		KeyCode::Esc => {
			state.focus = TuiFocus::Tools;
		}
		KeyCode::Tab => {
			state.focus = if state.tree_visible {
				TuiFocus::Tree
			} else {
				TuiFocus::Tools
			};
		}
		_ => {}
	}
}

fn handle_tree_input(state: &mut TuiState, code: KeyCode) {
	match code {
		KeyCode::Up | KeyCode::Char('k') => {
			state.tree_state.select_previous();
		}
		KeyCode::Down | KeyCode::Char('j') => {
			state.tree_state.select_next();
		}
		KeyCode::Enter => {
			if let Some(idx) = state.tree_state.selected() {
				if let Some(entry) = state.tree_entries.get(idx) {
					let path = entry.path.clone();
					state.command_tx.send_blocking(path).ok();
					state.response_text = "Loading...".into();
				}
			}
		}
		KeyCode::Tab => {
			state.focus = TuiFocus::Tools;
		}
		KeyCode::Char(':') | KeyCode::Char('/') => {
			state.focus = TuiFocus::Command;
		}
		_ => {}
	}
}

fn handle_tools_input(state: &mut TuiState, code: KeyCode) {
	match code {
		KeyCode::Up | KeyCode::Char('k') => {
			state.tool_index = state.tool_index.saturating_sub(1);
		}
		KeyCode::Down | KeyCode::Char('j') => {
			state.tool_index = state.tool_index.saturating_add(1);
			// clamping happens at render time when we know the count
		}
		KeyCode::Tab => {
			state.focus = TuiFocus::Command;
		}
		KeyCode::Char(':') | KeyCode::Char('/') => {
			state.focus = TuiFocus::Command;
		}
		_ => {}
	}
}

// ── Response polling ─────────────────────────────────────────────────

fn tui_poll_responses(mut state: ResMut<TuiState>) {
	// Drain all pending responses, keeping the latest
	while let Ok(text) = state.response_rx.try_recv() {
		state.response_text = text;
	}

	// Handle deferred quit
	if state.command_input == "quit" {
		state.command_input.clear();
	}
}

// ── Tree sync ────────────────────────────────────────────────────────

fn tui_sync_tree(
	mut state: ResMut<TuiState>,
	servers: Query<Entity, With<TuiServer>>,
	trees: Query<&RouteTree>,
	ancestors: Query<&ChildOf>,
) {
	let Ok(server_entity) = servers.single() else {
		return;
	};

	// Walk to root ancestor
	let mut root = server_entity;
	while let Ok(child_of) = ancestors.get(root) {
		root = child_of.parent();
	}

	let Ok(tree) = trees.get(root) else {
		return;
	};

	// Rebuild tree entries
	let mut entries = Vec::new();
	collect_tree_entries(tree, &mut entries, 0);
	state.tree_entries = entries;
}

fn collect_tree_entries(
	tree: &RouteTree,
	entries: &mut Vec<TreeEntry>,
	depth: usize,
) {
	if let Some(node) = tree.node() {
		let path_str = node.path().annotated_route_path().to_string();
		let label = match node {
			RouteNode::Card(_) => {
				let name = path_str.rsplit('/').next().unwrap_or(&path_str);
				if name.is_empty() { "/" } else { name }.to_string()
			}
			RouteNode::Tool(tool) => {
				let name = path_str.rsplit('/').next().unwrap_or(&path_str);
				let short_in = short_type_name(tool.meta.input().type_name());
				let short_out = short_type_name(tool.meta.output().type_name());
				format!("{name} ({short_in} → {short_out})")
			}
		};
		entries.push(TreeEntry {
			label,
			path: path_str,
			depth,
		});
	}
	for child in &tree.children {
		collect_tree_entries(child, entries, depth + 1);
	}
}

/// Extracts the short name from a fully qualified type name.
fn short_type_name(full: &str) -> &str {
	full.rsplit("::").next().unwrap_or(full)
}


// ── Drawing ──────────────────────────────────────────────────────────

fn tui_draw_system(
	mut context: ResMut<RatatuiContext>,
	state: Res<TuiState>,
	tool_map: Res<TuiToolMap>,
	servers: Query<Entity, With<TuiServer>>,
	trees: Query<&RouteTree>,
	ancestors: Query<&ChildOf>,
) -> Result {
	// Gather tool nodes from the route tree
	let tool_nodes: Vec<ToolNode> = servers
		.single()
		.ok()
		.and_then(|server_entity| {
			let mut root = server_entity;
			while let Ok(child_of) = ancestors.get(root) {
				root = child_of.parent();
			}
			trees.get(root).ok()
		})
		.map(|tree| tree.flatten_tool_nodes().into_iter().cloned().collect())
		.unwrap_or_default();

	let state_ref = &*state;
	let tool_map_ref = &*tool_map;

	context.draw(|frame| {
		render_tui(frame, state_ref, tool_map_ref, &tool_nodes);
	})?;

	Ok(())
}

fn render_tui(
	frame: &mut Frame,
	state: &TuiState,
	tool_map: &TuiToolMap,
	tool_nodes: &[ToolNode],
) {
	let area = frame.area();

	// Vertical split: content area + command palette
	let outer = Layout::vertical([Constraint::Min(3), Constraint::Length(3)])
		.split(area);

	let content_area = outer[0];
	let command_area = outer[1];

	// Decide tree visibility based on width and toggle
	let show_tree = state.tree_visible && area.width >= TREE_AUTO_SHOW_WIDTH;

	let (tree_area, main_area) = if show_tree {
		let chunks = Layout::horizontal([
			Constraint::Length(TREE_PANEL_WIDTH),
			Constraint::Min(20),
		])
		.split(content_area);
		(Some(chunks[0]), chunks[1])
	} else {
		(None, content_area)
	};

	// Render tree panel
	if let Some(tree_rect) = tree_area {
		render_tree_panel(frame, state, tree_rect);
	}

	// Render main panel (response + tools)
	render_main_panel(frame, state, tool_map, tool_nodes, main_area);

	// Render command palette
	render_command_palette(frame, state, command_area);
}

fn render_tree_panel(frame: &mut Frame, state: &TuiState, area: TuiRect) {
	let focused = state.focus == TuiFocus::Tree;
	let border_style = if focused {
		TuiStyle::default().fg(TuiColor::Yellow)
	} else {
		TuiStyle::default().fg(TuiColor::DarkGray)
	};

	let items: Vec<ListItem> = state
		.tree_entries
		.iter()
		.map(|entry| {
			let indent = "  ".repeat(entry.depth);
			let icon = if entry.path.contains('/') {
				"  "
			} else {
				"▸ "
			};
			ListItem::new(format!("{indent}{icon}{}", entry.label))
		})
		.collect();

	let block = Block::default()
		.title(" Cards ")
		.borders(Borders::ALL)
		.border_style(border_style);

	let highlight_style = TuiStyle::default()
		.bg(TuiColor::DarkGray)
		.fg(TuiColor::White)
		.bold();

	let list = List::new(items)
		.block(block)
		.highlight_style(highlight_style)
		.highlight_symbol("▸ ");

	// Clone the list state so we can pass it mutably
	let mut list_state = state.tree_state.clone();
	StatefulWidget::render(list, area, frame.buffer_mut(), &mut list_state);
}

fn render_main_panel(
	frame: &mut Frame,
	state: &TuiState,
	tool_map: &TuiToolMap,
	tool_nodes: &[ToolNode],
	area: TuiRect,
) {
	let focused_tools = state.focus == TuiFocus::Tools;

	// Split main panel: response text on top, tools on bottom
	let tool_height = if tool_nodes.is_empty() {
		0
	} else {
		(tool_nodes.len() as u16 + 2).min(area.height / 3).max(3)
	};

	let chunks =
		Layout::vertical([Constraint::Min(3), Constraint::Length(tool_height)])
			.split(area);

	let response_area = chunks[0];
	let tools_area = chunks[1];

	// Response text
	let response_block = Block::default()
		.title(" Response ")
		.borders(Borders::ALL)
		.border_style(TuiStyle::default().fg(TuiColor::DarkGray));

	let response = Paragraph::new(state.response_text.as_str())
		.block(response_block)
		.wrap(Wrap { trim: false });

	frame.render_widget(response, response_area);

	// Tools panel
	if !tool_nodes.is_empty() {
		let tools_border_style = if focused_tools {
			TuiStyle::default().fg(TuiColor::Yellow)
		} else {
			TuiStyle::default().fg(TuiColor::DarkGray)
		};

		let tools_block = Block::default()
			.title(" Tools ")
			.borders(Borders::ALL)
			.border_style(tools_border_style);

		let inner = tools_block.inner(tools_area);
		frame.render_widget(tools_block, tools_area);

		// Render each tool
		let tool_lines = Layout::vertical(
			tool_nodes
				.iter()
				.map(|_| Constraint::Length(1))
				.collect::<Vec<_>>(),
		)
		.split(inner);

		for (idx, tool) in tool_nodes.iter().enumerate() {
			if idx >= tool_lines.len() {
				break;
			}
			let tool_area = tool_lines[idx];
			let is_focused = focused_tools && idx == state.tool_index;

			let path_str = tool.path.annotated_route_path().to_string();
			let tool_name = path_str.rsplit('/').next().unwrap_or(&path_str);

			let input_type_id = tool.meta.input().type_id();
			let input_type_name = tool.meta.input().type_name();
			let output_type_name = tool.meta.output().type_name();
			let short_in = short_type_name(input_type_name);
			let short_out = short_type_name(output_type_name);

			// Label area and input widget area
			let tool_chunks = Layout::horizontal([
				Constraint::Length((tool_name.len() as u16) + 2),
				Constraint::Length(
					(short_in.len() + short_out.len() + 5) as u16,
				),
				Constraint::Min(8),
			])
			.split(tool_area);

			let name_style = if is_focused {
				TuiStyle::default().fg(TuiColor::Yellow).bold()
			} else {
				TuiStyle::default().fg(TuiColor::White)
			};
			let type_style = TuiStyle::default().fg(TuiColor::DarkGray);

			frame.render_widget(
				Span::styled(format!(" {tool_name}"), name_style),
				tool_chunks[0],
			);
			frame.render_widget(
				Span::styled(format!(" {short_in} → {short_out}"), type_style),
				tool_chunks[1],
			);

			// Render the appropriate input widget
			let current_value = state
				.tool_inputs
				.get(&path_str)
				.map(|s| s.as_str())
				.unwrap_or("");

			// Look up the renderer by the tool's input TypeId
			if let Some(renderer) = tool_map.0.get(&input_type_id) {
				renderer(frame, tool_chunks[2], current_value, is_focused);
			} else {
				// Fallback: generic display
				let style = if is_focused {
					TuiStyle::default().fg(TuiColor::Yellow)
				} else {
					TuiStyle::default().fg(TuiColor::DarkGray)
				};
				let label = if is_focused { "▸ [Run]" } else { "  [Run]" };
				frame.render_widget(Span::styled(label, style), tool_chunks[2]);
			}
		}
	}
}

fn render_command_palette(frame: &mut Frame, state: &TuiState, area: TuiRect) {
	let focused = state.focus == TuiFocus::Command;
	let border_style = if focused {
		TuiStyle::default().fg(TuiColor::Yellow)
	} else {
		TuiStyle::default().fg(TuiColor::DarkGray)
	};

	let block = Block::default()
		.title(" Command ")
		.borders(Borders::ALL)
		.border_style(border_style);

	let inner = block.inner(area);
	frame.render_widget(block, area);

	let prompt = if focused { "> " } else { "  " };
	let text = format!("{prompt}{}", state.command_input);
	let style = if focused {
		TuiStyle::default().fg(TuiColor::White)
	} else {
		TuiStyle::default().fg(TuiColor::DarkGray)
	};

	frame.render_widget(Span::styled(text, style), inner);

	// Show cursor at end of input when focused
	if focused {
		let cursor_x = inner.x + 2 + state.command_input.len() as u16;
		let cursor_y = inner.y;
		if cursor_x < inner.right() {
			frame.set_cursor_position((cursor_x, cursor_y));
		}
	}
}
