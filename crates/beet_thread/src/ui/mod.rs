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
use beet_net::prelude::*;
use beet_router::prelude::*;

/// The store key the crate-shipped `CreatePostForm.bsx` is seeded at: under a
/// `templates/` root so [`TemplateDir`] derives its `CreatePostForm` module path.
const CREATE_POST_FORM_KEY: &str = "templates/CreatePostForm.bsx";

/// Registers the [`ThreadWindow`] -> [`Document`] projection and the reactive
/// UI types. Pairs with `beet_ui`'s [`CharcellTuiPlugin`] (or any renderer that
/// drives the document chain) and the [`ThreadPlugin`].
#[derive(Default)]
pub struct ThreadUiPlugin;

impl Plugin for ThreadUiPlugin {
	fn build(&self, app: &mut App) {
		// The crate-shipped `CreatePostForm.bsx` registers through the same
		// store-backed path as site templates: seed its compile-time bytes into an
		// in-memory `BlobStore` and load that with a `TemplateDir`. Spawned from
		// `Startup` (not here) so `RouterPlugin`'s `TemplateDir::register_on_insert`
		// observer is built first; the register is async, so a render-at-startup path
		// settles tasks before the first render (see `register_create_post_form`).
		app.init_plugin::<RouterPlugin>()
			.add_systems(Startup, register_create_post_form);
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

/// Seed the crate-shipped [`CREATE_POST_FORM_BSX`] bytes into an embedded
/// in-memory [`BlobStore`] and load it with a [`TemplateDir`], registering
/// `CreatePostForm` through the same store path as a site's own templates (so a
/// site shipping its own `CreatePostForm.bsx` overrides it). Runs at [`Startup`],
/// after [`RouterPlugin`]'s [`TemplateDir::register_on_insert`] observer is built;
/// that observer reads the store off the runtime (async), so a render-at-startup
/// path must settle async tasks before the first render resolves the form.
fn register_create_post_form(mut commands: Commands) {
	let store = BlobStore::new(InMemoryStore::new_seeded([(
		SmolPath::from(CREATE_POST_FORM_KEY),
		CREATE_POST_FORM_BSX.into(),
	)]));
	commands.spawn((store, TemplateDir::new("templates")));
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The crate-shipped `CreatePostForm` is registered through the embedded blob
	/// store (`ThreadUiPlugin`'s `Startup` seed + `TemplateDir`), not a direct
	/// `insert_source`: once the async store read settles, the registry holds it.
	#[beet_core::test]
	async fn registers_create_post_form_through_store() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_plugin::<ThreadUiPlugin>();
		// not yet: registration is an async task spawned from `Startup`
		app.world()
			.get_resource::<BsxTemplateRegistry>()
			.is_some_and(|registry| registry.contains(CREATE_POST_FORM_TEMPLATE))
			.xpect_false();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		app.world()
			.resource::<BsxTemplateRegistry>()
			.contains(CREATE_POST_FORM_TEMPLATE)
			.xpect_true();
	}
}
