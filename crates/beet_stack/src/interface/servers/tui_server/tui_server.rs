//! A TUI server that renders an interactive terminal interface for
//! browsing cards and invoking tools.
//!
//! Uses [`bevy_ratatui`] for terminal rendering and delegates command
//! dispatch to an async loop, communicating with the draw systems via
//! channels managed by [`TuiPlugin`].
use crate::prelude::*;
use beet_core::prelude::*;

/// A TUI server [`Bundle`].
///
/// On spawn, starts an async dispatch loop that receives commands from
/// the [`TuiState`] command channel, dispatches each as a [`Request`]
/// through the owning entity's tool pipeline, and sends the response
/// body back for display in the terminal.
///
/// Requires [`TuiPlugin`] to be added to the app. Typically combined
/// with a [`default_interface`] and child tools/cards:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((
///         MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
///             std::time::Duration::from_secs_f32(1. / 60.),
///         )),
///         LogPlugin::default(),
///         StackPlugin,
///         TuiPlugin,
///     ));
///     app.world_mut().spawn((
///         tui_server(),
///         default_interface(),
///         children![
///             increment(FieldRef::new("count")),
///             card("about"),
///         ],
///     ));
///     app.run();
/// }
/// ```
pub fn tui_server() -> impl Bundle {
	(
		TuiServer,
		OnSpawn::new(|entity| {
			entity.run_async(async |entity| -> Result {
				let channel: TuiCommandChannel =
					entity.world().resource::<TuiCommandChannel>().await;
				let cmd_rx = channel.command_rx.clone();
				let res_tx = channel.response_tx.clone();

				// Send initial help text
				let help_req = Request::from_cli_str("--help")?;
				let help_res: Response = entity.call(help_req).await?;
				let help_body = help_res.unwrap_str().await;
				let _: Result<(), _> = res_tx.send(help_body).await;

				// Dispatch loop: receive commands, call the tool pipeline,
				// send response bodies back for rendering
				while let Ok(command) = cmd_rx.recv().await {
					let trimmed = command.trim().to_string();
					if trimmed == "exit" || trimmed == "quit" {
						entity.world().write_message(AppExit::Success);
						break;
					}

					match Request::from_cli_str(&trimmed) {
						Ok(req) => {
							match entity.call::<Request, Response>(req).await {
								Ok(res) => {
									let body = res.unwrap_str().await;
									let _: Result<(), _> =
										res_tx.send(body).await;
								}
								Err(err) => {
									let msg = format!("Error: {err}");
									let _: Result<(), _> =
										res_tx.send(msg).await;
								}
							}
						}
						Err(err) => {
							let msg = format!("Parse error: {err}");
							let _: Result<(), _> = res_tx.send(msg).await;
						}
					}
				}

				Ok(())
			});
		}),
	)
}
