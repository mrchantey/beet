//! TUI keyboard input handling.
//!
//! Routes key events to the correct handler based on which panel
//! currently has [`TuiFocus`].

use super::*;
use beet_core::prelude::*;
use bevy_ratatui::event::KeyMessage;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyModifiers;

pub(super) fn tui_input_system(
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
