//! TUI server that renders cards in a terminal interface.
//!
//! Includes a [`tui_render_tool`] as a child entity so that cards
//! render via [`TuiRenderer`] and persist as [`CurrentCard`] for
//! the draw system.
use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a TUI server [`Bundle`].
///
/// On spawn, parses CLI arguments into an initial [`Request`] and
/// dispatches it through the owning entity's tool pipeline.
///
/// Includes a [`tui_render_tool`] child entity for stateful card
/// rendering. Typically combined with a [`default_router`] and
/// child cards/tools:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins(TuiPlugin);
///     app.world_mut().spawn((
///         default_router(),
///         tui_server(),
///         children![
///             card("home", || Heading1::with_text("Welcome")),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn tui_server() -> impl Bundle {
	(
		OnSpawn::insert_child(tui_render_tool()),
		OnSpawn::new_async(async |entity| -> Result {
			// Dispatch CLI args as the initial request, rendering the
			// root content when no args are provided.
			let initial_req = Request::from_cli_args(CliArgs::parse_env())?;
			entity
				.call::<_, Response>(initial_req)
				.await?
				.into_result()
				.await?;


			Ok(())
		}),
	)
}
