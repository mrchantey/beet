//! A TUI server that renders a terminal interface for browsing cards
//! and invoking tools, using [`bevy_ratatui`] for rendering.
mod tui_plugin;
pub use tui_plugin::*;
mod tui_server;
pub use tui_server::*;
