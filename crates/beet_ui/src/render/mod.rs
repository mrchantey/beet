//! Rendering an entity tree to bytes: one walk, many formats.
//!
//! A [`NodeRenderer`] walks the tree rooted at an entity (through
//! [`NodeWalker`], which reads it through [`RenderTreeQuery`], so every
//! [`Portal`] renders the tree it transcludes in place) and writes one format. Each format is a [`RenderTarget`], and every target a world renders
//! to lives in one registry, [`RenderTargets`], the one path every render takes.
//!
//! Every render answers a request: an http request, a cli command, a
//! syndication route's in-process `Request::get`, a publish step, a test's
//! [`RequestParts::default()`]. [`RenderTargets::render`] negotiates the format
//! from its `Accept` ([`RenderTargets::negotiate`]), the default media type
//! answering an absent header or a wildcard and any unregistered text type
//! falling back to plain text, and the target reads the same request off its
//! [`RenderContext`]. A caller wanting a particular format renders a request
//! accepting it, so a direct render can never drift from what a client asking
//! for the same type receives.
//!
//! [`RenderPlugin`] registers the built-in targets: [`HtmlRenderer`],
//! [`MarkdownRenderer`], [`PlainTextRenderer`], and with their features
//! [`AnsiTermRenderer`] (`style`) and the serialized scene of
//! [`TemplateRenderer`] (`template_serde`, json and postcard).
//!
//! # Registering a target
//!
//! A downstream crate registers its own target through the same call the
//! built-ins use, from its plugin:
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! # use beet_ui::prelude::*;
//! #[derive(Clone)]
//! struct Shout;
//!
//! impl NodeRenderer for Shout {
//! 	fn render(
//! 		&mut self,
//! 		cx: &mut RenderContext,
//! 	) -> Result<MediaBytes, RenderError> {
//! 		let mut text = PlainTextRenderer::default();
//! 		cx.walk(&mut text);
//! 		MediaBytes::new_string(
//! 			MediaType::other("text/x-shout"),
//! 			text.into_string().to_uppercase(),
//! 		)
//! 		.xok()
//! 	}
//! }
//!
//! impl RenderTarget for Shout {
//! 	fn media_types(&self) -> Vec<MediaType> {
//! 		vec![MediaType::other("text/x-shout")]
//! 	}
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(RenderPlugin).register_render_target(Shout);
//! let world = app.world_mut();
//! let page = world.spawn(rsx! { <p>"hello"</p> }).id();
//! let request = RequestParts::default()
//! 	.with_accept(MediaType::other("text/x-shout"));
//! RenderTargets::render(world, page, &request)
//! 	.unwrap()
//! 	.to_string()
//! 	.xpect_eq("HELLO\n");
//! ```
//!
//! The registered instance is cloned for every render, so its configuration
//! is fixed at registration. A target registered later answers its media
//! types ahead of an earlier one, which is how a built-in is replaced.
//!
//! [`Portal`]: crate::prelude::Portal
//! [`RenderTreeQuery`]: crate::prelude::RenderTreeQuery
//! [`RequestParts::default()`]: beet_net::prelude::RequestParts
#[cfg(feature = "style")]
mod charcell;
#[cfg(feature = "style")]
pub use charcell::*;
// the html-rendering target and its utilities; the shared
// `node_walker`/`node_renderer` substrate
// (also used by markdown/ansi/charcell) stays here in `render/`.
mod html;
pub use html::*;
// the DOM render target: the browser's document as a sink over the same walk,
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
mod style_map;
pub use style_map::*;
mod render_target;
pub use render_target::*;
mod plaintext;
pub use plaintext::*;
mod node_renderer;
pub use node_renderer::*;
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
