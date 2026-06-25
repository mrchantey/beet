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
//! ([`thread_view`] / [`input`]); [`scene`] supplies a local charcell host shell.
//! `beet_ui` never depends on `beet_thread`; this layer is additive, behind the
//! `ui` feature.

mod input;
pub use input::*;
mod of_thread;
pub use of_thread::*;
mod scene;
pub use scene::*;
mod thread_view;
pub use thread_view::*;

use beet_core::prelude::*;

/// Registers the [`ThreadWindow`] -> [`Document`] projection and the reactive
/// UI types. Pairs with `beet_ui`'s [`CharcellTuiPlugin`] (or any renderer that
/// drives the document chain) and the [`ThreadPlugin`].
#[derive(Default)]
pub struct ThreadUiPlugin;

impl Plugin for ThreadUiPlugin {
	fn build(&self, app: &mut App) {
		// TODO(cli-rework): interim template loading. Register the crate-shipped
		// `CreatePostForm.bsx` source so `create_post_form_on_add` can resolve it.
		// Swap this one line for the blob-store load once crate-shipped templates
		// travel through the blob store (`.agents/plans/cli-rework.md`).
		app.world_mut()
			.get_resource_or_init::<BsxTemplateRegistry>()
			.insert_source(CREATE_POST_FORM_TEMPLATE, CREATE_POST_FORM_BSX)
			.unwrap();
		app.register_type::<ThreadView>()
			.register_type::<ThreadScroll>()
			.register_type::<CreatePostForm>()
			// the thread<->UI relationship binding views/forms to their thread
			.register_type::<OfThread>()
			.register_type::<ThreadItems>()
			.register_type::<UserInput>()
			// the local charcell host shells, declarable from markup
			.register_template::<TuiThreadChat>()
			.register_template::<TuiThreadTranscript>()
			// project each window into its views' documents, then pin to the bottom
			.add_systems(
				Update,
				(project_window_to_document, follow_thread_scroll).chain(),
			);
		// The form's empty-on-submit (`ClearOnSubmit`) and initial focus
		// (`FocusOnAdd`) are generic `beet_ui` markers spread in `CreatePostForm.bsx`;
		// surface scoping is the host's job (it carries `RenderSurface(self)`).
	}
}
