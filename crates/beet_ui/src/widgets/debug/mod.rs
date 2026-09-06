//! Widgets that surface the running app to whoever is working on it, rather
//! than to its users: [`RenderConsole`], the in-page log panel.
pub(in crate::widgets) mod render_console;

// only the widget is public; its style consts (`CONSOLE_*`) stay `pub(crate)`,
// reached prefixed as `render_console::CONSOLE_INFO` within the crate.
pub use render_console::RenderConsole;
