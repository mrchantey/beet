//! Rendering an entity tree to a document: one walk, many formats.
//!
//! A [`DocRenderer`] walks the tree rooted at an entity (through
//! [`NodeWalker`], which reads it through [`RenderTreeQuery`], so every
//! [`Portal`] renders the tree it transcludes in place) and writes it as a
//! document of one format, even plain text being a document of a single text
//! node. Every renderer a world renders to lives in one registry,
//! [`DocRenderers`], the one path every render takes. A live surface, the DOM
//! or a charcell buffer, is painted rather than rendered to a document, and
//! is no doc renderer.
//!
//! Every render answers a request: an http request, a cli command, a
//! syndication route's in-process `Request::get`, a publish step, a test's
//! [`RequestParts::default()`]. [`DocRenderers::render`] negotiates the format
//! from its `Accept` ([`DocRenderers::negotiate`]), the default media type
//! answering an absent header or a wildcard and any unregistered text type
//! falling back to plain text, and the renderer reads the same request off its
//! [`RenderContext`]. A caller wanting a particular format renders a request
//! accepting it, so a direct render can never drift from what a client asking
//! for the same type receives.
//!
//! [`RenderPlugin`] registers the built-in renderers: [`HtmlRenderer`],
//! [`XmlRenderer`], [`MarkdownRenderer`], [`PlainTextRenderer`], and with
//! their features [`AnsiTermRenderer`] (`style`) and the serialized scene of
//! [`TemplateRenderer`] (`scene`, json and postcard).
//!
//! # Registering a renderer
//!
//! A downstream crate registers its own renderer through the same call the
//! built-ins use, from its plugin:
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! # use beet_ui::prelude::*;
//! #[derive(Clone)]
//! struct Shout;
//!
//! impl DocRenderer for Shout {
//! 	fn media_types(&self) -> Vec<MediaType> {
//! 		vec![MediaType::other("text/vnd.shout")]
//! 	}
//!
//! 	fn render(
//! 		&mut self,
//! 		cx: &mut RenderContext,
//! 	) -> Result<MediaBytes, RenderError> {
//! 		let mut text = PlainTextRenderer::default();
//! 		cx.walk(&mut text);
//! 		MediaBytes::new_string(
//! 			MediaType::other("text/vnd.shout"),
//! 			text.into_string().to_uppercase(),
//! 		)
//! 		.xok()
//! 	}
//! }
//!
//! impl Installer for Shout {
//! 	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(RenderPlugin).install(Shout);
//! let world = app.world_mut();
//! let page = world.spawn(rsx! { <p>"hello"</p> }).id();
//! let request = RequestParts::default()
//! 	.with_accept(MediaType::other("text/vnd.shout"));
//! DocRenderers::render(world, page, &request)
//! 	.unwrap()
//! 	.to_string()
//! 	.xpect_eq("HELLO\n");
//! ```
//!
//! The registered instance is cloned for every render, so its configuration
//! is fixed at registration. A renderer registered later answers its media
//! types ahead of an earlier one, which is how a built-in is replaced.
//!
//! [`Portal`]: crate::prelude::Portal
//! [`RenderTreeQuery`]: crate::prelude::RenderTreeQuery
//! [`RequestParts::default()`]: beet_net::prelude::RequestParts
#[cfg(feature = "style")]
mod charcell;
#[cfg(feature = "style")]
pub use charcell::*;
// the html renderer and its utilities; the shared
// `node_walker`/`render_context` substrate
// (also used by markdown/ansi/charcell) stays here in `render/`.
mod html;
pub use html::*;
// the DOM surface: the browser's document as a sink over the same walk,
// painted once and patched each frame; there is nothing to paint into
// anywhere else, so it is target-gated rather than featured.
#[cfg(target_arch = "wasm32")]
mod dom;
#[cfg(target_arch = "wasm32")]
pub use dom::*;
// the served page's pre-boot pieces: rendered natively into the page, read
// back by the browser world once it has adopted it.
mod pre_boot;
pub use pre_boot::*;
// an element whose children the browser fills in, which the DOM sink
// adopts whole
mod foreign_children;
pub use foreign_children::*;
mod doc_renderers;
pub use doc_renderers::*;
mod plaintext;
pub use plaintext::*;
mod render_context;
pub use render_context::*;
mod node_walker;
pub use node_walker::*;
mod text_render_state;
pub use text_render_state::*;
mod markdown;
pub use markdown::*;
#[cfg(feature = "style")]
mod ansi_term;
#[cfg(feature = "style")]
pub use ansi_term::*;
