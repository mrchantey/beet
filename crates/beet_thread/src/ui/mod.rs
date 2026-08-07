//! Agnostic reactive chat UI for a [`ThreadWindow`], built on `beet_ui`.
//!
//! The pipeline is two hops, each renderer-agnostic:
//! 1. [`project_window_to_document`] projects a thread's [`ThreadWindow`] into a
//!    structured [`Document`] (`{ "posts": [{ id, author, text }, ..] }`),
//! 2. [`ThreadView`] renders that document reactively: a scroll container whose
//!    rows are a *keyed* [`ReactiveChildren`] over the `posts` field, each row's
//!    body bound through a [`FieldRef`] so streamed text flows in without
//!    rebuilding the row.
//!
//! Keying on the post id means an appended post reuses every settled row's
//! entity and binding, and a growing in-progress body re-syncs that row's bound
//! [`Value`] rather than respawning it. The view + composer are host-agnostic
//! ([`thread_view`] / [`input`]); hosting is the server's job, and [`layout`]
//! supplies the minimal document shell its routes render into.
//! `beet_ui` never depends on `beet_thread`; this layer is additive, behind the
//! `ui` feature.

mod input;
pub use input::*;
mod layout;
pub use layout::*;
mod of_thread;
pub use of_thread::*;
mod thread_view;
pub use thread_view::*;

use beet_core::prelude::*;
use beet_router::prelude::*;

/// Registers the [`ThreadWindow`] -> [`Document`] projection and the reactive
/// UI types. Pairs with `beet_ui`'s [`CharcellTuiPlugin`] (or any renderer that
/// drives the document chain) and the [`ThreadPlugin`].
#[derive(Default)]
pub struct ThreadUiPlugin;

impl Plugin for ThreadUiPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<RouterPlugin>();
		app.register_type::<ThreadView>()
			.register_type::<ThreadScroll>()
			.register_type::<CreatePostForm>()
			// the thread<->UI relationship binding views/forms to their thread
			.register_type::<OfThread>()
			.register_type::<ThreadItems>()
			.register_type::<UserInput>()
			// the document shell a thread scene's routes are wrapped in
			.register_type::<ThreadLayout>()
			// project each window into its views' documents, then pin to the bottom
			.add_systems(
				Update,
				(project_window_to_document, follow_thread_scroll).chain(),
			);
		// The form's empty-on-submit (`ClearOnSubmit`) and initial focus
		// (`FocusOnAdd`) are generic `beet_ui` markers spread by the form's rust
		// template (see `input.rs`); surface scoping is the host's job (it
		// carries `RenderSurface(self)`).
	}
}
