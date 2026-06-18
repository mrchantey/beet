// std-only: the browser-style Navigator drives the live-render pipeline
// (`parse_page`/`build_live_page`/`bind_surface_page`), which needs beet_ui.
#[cfg(feature = "std")]
mod navigator;
#[cfg(feature = "std")]
pub use navigator::*;
// std-only: the navigation-failure error page (a beet_ui `#[template]`).
#[cfg(feature = "std")]
mod error_page;
#[cfg(feature = "std")]
pub use error_page::*;
mod navigate;
pub use navigate::*;
// std-only: drives navigation into the beet_ui render-media pipeline.
#[cfg(feature = "std")]
mod navigator_plugin;
#[cfg(feature = "std")]
pub use navigator_plugin::*;
// std-only: renders the active route into a persistent DoubleBuffer (needs beet_ui).
#[cfg(feature = "std")]
mod live_page;
#[cfg(feature = "std")]
pub use live_page::*;
// std-only: link classification + OnOpenLink (needs beet_ui ElementQuery/LinkView).
#[cfg(feature = "std")]
mod open_link;
#[cfg(feature = "std")]
pub use open_link::*;
// terminal-only: the live-TUI server entry (needs beet_ui StdioTerminal).
#[cfg(feature = "tui")]
mod tui_server;
#[cfg(feature = "tui")]
pub use tui_server::*;
// the multi-tenant SSH-TUI server (needs the russh server transport).
#[cfg(feature = "ssh")]
mod ssh_tui_server;
#[cfg(feature = "ssh")]
pub use ssh_tui_server::*;
