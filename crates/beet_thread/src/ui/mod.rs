//! Agnostic reactive chat UI for a [`ThreadRecords`](crate::prelude::ThreadRecords), built on `beet_ui`.
//!
//! [`ThreadView`] renders a window's records as a scroll container holding one
//! row entity a loaded post, each row its own [`Document`] its fields bind to
//! through a [`FieldRef`]. The view draws only what changed since it last
//! looked ([`ThreadRecords::changed_since`](crate::prelude::ThreadRecords::changed_since)),
//! so an appended post spawns one row and a growing in-progress body writes
//! that row's document alone, however long the thread. The view + composer are host-agnostic
//! ([`ThreadView`] / [`CreatePostForm`]); hosting is the server's job, and a thread
//! scene wraps its routes in `beet_router`'s chrome-free `AppShell`.
//! `beet_ui` never depends on `beet_thread`; this layer is additive, behind the
//! `ui` feature.

// the served surface's end-to-end suite
#[cfg(test)]
mod chat_surface;
mod input;
#[cfg(test)]
mod test_ext;
pub use input::*;
mod of_thread;
pub use of_thread::*;
mod thread_view;
pub use thread_view::*;

use beet_core::prelude::*;
use beet_router::prelude::*;

/// Registers the views' row drawing, their scroll and the reactive UI
/// types. Pairs with `beet_ui`'s [`CharcellTuiPlugin`](beet_ui::prelude::CharcellTuiPlugin) (or any renderer that
/// drives the document chain) and the [`ThreadPlugin`](crate::prelude::ThreadPlugin).
#[derive(Default)]
pub struct ThreadUiPlugin;

impl Plugin for ThreadUiPlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<RouterPlugin>();
		app.register_type::<ThreadView>()
			.register_type::<ThreadScroll>()
			.register_type::<CreatePostForm>()
			// the thread<->UI relationship binding views/forms to their thread
			.register_type::<OfThread>()
			.register_type::<ThreadItems>()
			// the composer posts on submit and stops on Escape, which reads
			// the keyboard
			.install_types::<bevy::input::InputPlugin>()
			.add_observer(input::post_on_submit)
			.add_systems(Update, input::stop_on_escape);
		// draw what changed in each view's rows, pin to the bottom or hold a
		// scrolled up reader's place, and read older posts near the top
		app.add_systems(
			Update,
			(sync_thread_rows, follow_thread_scroll, read_older_on_scroll)
				.chain(),
		);
	}
}
