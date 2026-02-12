//! TUI drawing, tree synchronization, and response polling.
//!
//! Handles the rendering of the tree panel, main panel (response text
//! and tool widgets), and command palette. Also syncs the card tree
//! from the [`RouteTree`] and polls async responses.

use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::Frame;
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
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Wrap;

// ── Response polling ─────────────────────────────────────────────────

pub(super) fn tui_poll_responses(mut state: ResMut<TuiState>) {
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

pub(super) fn tui_sync_tree(
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

pub(super) fn tui_draw_system(
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
