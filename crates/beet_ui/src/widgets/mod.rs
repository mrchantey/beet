//! Reusable `#[template]` function components.
//!
//! These emit *semantic* classes via [`Classes`](crate::token::Classes)
//! (never `class="…"` strings); the active rule set (Material Design 3 via
//! `MaterialStylePlugin` today) maps those classes to design tokens.
//!
//! Gated behind the `scene` feature; rendering targets and styling come from
//! the same DOM + rule machinery as parsed HTML.
//!
//! **Reactive substrate.** State lives in documents:
//! [`TypedFieldRef`](beet_core::prelude::TypedFieldRef) for a single typed atom and
//! [`ReactiveChildren`](beet_core::prelude::ReactiveChildren) for a list field that
//! materializes one child per item. The full loop — native event then document
//! mutation then change-detected rebuild — is proven by `native_event_drives_list`
//! in `document/reactive_children.rs`, with zero render-target coupling.
//!
//! A DOM widget or a `BlobStore`-backed list is a render-target / storage
//! binding layered on top, not a gap in the substrate: a render backend triggers
//! the native events (see `input/pointer.rs`), and an async store can sync
//! `BlobStore::list()` into a `Vec<_>` field via
//! [`AsyncWorld`](beet_core::prelude::AsyncWorld) when that integration is wanted.

#[cfg(feature = "net")]
mod analytics;
mod button;
// `code_snippet` calls `SyntaxHighlighting`, which is native-only (tree-sitter),
// so mirror its `not(wasm32)` gate.
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
mod code_snippet;
mod color_scheme;
mod error_text;
mod footer;
mod form_controls;
mod head;
mod header;
mod layout;
mod preflight;
mod sidebar;
#[cfg(feature = "style")]
mod stylesheet;
mod table;

#[cfg(feature = "net")]
pub use analytics::*;
pub use button::*;
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
pub use code_snippet::*;
pub use color_scheme::*;
pub use error_text::*;
pub use footer::*;
pub use form_controls::*;
pub use head::*;
pub use header::*;
pub use layout::*;
pub use preflight::*;
pub use sidebar::*;
#[cfg(feature = "style")]
pub use stylesheet::*;
pub use table::*;
// `button::Button` collides with the bevy_ui `Button` that leaks in via the
// `beet_core::prelude` glob below (under `bevy_default`); the explicit re-export pins
// the public `Button`, and downstream `prelude::Button`, to this crate's widget.
pub use button::Button;

use beet_core::prelude::*;

/// Registers the widget set by short type path, so a name-resolved tag (eg a
/// BSX `<Head/>` or a serialized scene) builds the widget. Added by
/// [`BsxDefaultsPlugin`](crate::prelude::BsxDefaultsPlugin).
pub fn widget_plugin(app: &mut App) {
	// `button::Button` is qualified so it resolves to this crate's widget rather
	// than the bevy_ui `Button` that leaks through `beet_core::prelude` when
	// `bevy_default` is co-enabled.
	app.register_template::<button::Button>()
		.register_template::<IconButton>()
		.register_template::<Link>()
		.register_template::<ColorSchemeScript>()
		.register_template::<ErrorText>()
		.register_template::<Footer>()
		.register_template::<TextField>()
		.register_template::<TextArea>()
		.register_template::<Select>()
		.register_template::<Form>()
		.register_template::<Head>()
		.register_template::<Header>()
		.register_template::<HtmlDocument>()
		.register_template::<PageLayout>()
		.register_template::<PageBreak>()
		.register_template::<ContentLayout>()
		.register_template::<Preflight>()
		.register_template::<Reset>()
		.register_template::<Sidebar>()
		.register_template::<SidebarScript>()
		.register_template::<MenuButton>()
		.register_template::<Table>();
	#[cfg(feature = "net")]
	app.register_template::<Analytics>();
	#[cfg(all(
		feature = "net",
		feature = "syntax_highlighting",
		not(target_arch = "wasm32")
	))]
	app.register_template::<CodeSnippet>();
	#[cfg(feature = "style")]
	app.register_template::<Stylesheet>();
}
