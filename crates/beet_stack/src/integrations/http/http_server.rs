//! HTTP server integration for beet_stack.
//!
//! Provides convenience bundles that combine a [`beet_net::HttpServer`]
//! with a [`mime_render_tool`] for content-negotiated rendering of
//! card content (HTML, markdown, JSON, or postcard based on the
//! `Accept` header).
//!
//! # Usage
//!
//! ```no_run
//! # use beet_core::prelude::*;
//! # use beet_stack::prelude::*;
//!
//! fn main() {
//!     let mut app = App::new();
//!     app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
//!     app.world_mut().spawn((
//!         default_router(),
//!         http_server(8080),
//!         children![
//!             card("", || Paragraph::with_text("welcome!")),
//!             card("about", || Paragraph::with_text("about")),
//!         ],
//!     ));
//!     async_ext::block_on(app.run_async());
//! }
//! ```
use crate::prelude::*;
use beet_core::prelude::*;

pub use beet_net::prelude::HttpServer;


/// Creates an HTTP server [`Bundle`] that listens on the given port.
///
/// Delegates to [`beet_net::HttpServer`] for the actual TCP/HTTP
/// handling and attaches a [`mime_render_tool`] for content-negotiated
/// rendering of card content.
///
/// Typically combined with a [`default_router`] and child tools:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
///     app.world_mut().spawn((
///         default_router(),
///         http_server(8080),
///         children![
///             card("", || Paragraph::with_text("welcome!")),
///             card("about", || Paragraph::with_text("about")),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn http_server(port: u16) -> impl Bundle {
	(
		HttpServer::new(port),
		OnSpawn::insert_child(mime_render_tool()),
	)
}

/// Creates an HTTP server bundle listening on all interfaces.
///
/// Use this for deployed servers that need to accept connections
/// from any network interface.
pub fn http_server_all_interfaces(port: u16) -> impl Bundle {
	(
		HttpServer::new_all_interfaces(port),
		OnSpawn::insert_child(mime_render_tool()),
	)
}
