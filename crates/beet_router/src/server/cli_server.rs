//! A single-shot CLI server that parses arguments from the environment,
//! dispatches them as a [`Request`], and streams the [`Response`] body
//! to stdout.
//!
//! Includes a [`mime_render_tool`] so that scene routes render their
//! content in the negotiated format for terminal output.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// A single-shot CLI server [`Bundle`].
///
/// On spawn, parses the process CLI arguments into a [`Request`],
/// calls the owning entity's tool pipeline, streams the response body
/// to stdout, and writes an [`AppExit`] message so the app terminates
/// with the appropriate exit code.
///
/// Includes a [`mime_render_tool`] for content-negotiated rendering
/// of scene route content in the terminal.
///
/// Typically combined with a [`default_router`] and some child
/// scene routes/tools to build a CLI application:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_router::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), BeetRouterPlugin));
///     app.world_mut().spawn((
///         default_router(),
///         cli_server(),
///         children![
///             scene_route("", || Name::new("welcome!")),
///             scene_route("about", || Name::new("about")),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn cli_server() -> impl Bundle {
	(
		OnSpawn::insert_child(mime_render_tool()),
		OnSpawn::new_async(async |entity| -> Result {
			let req = Request::from_cli_args(CliArgs::parse_env())?;
			let res: Response = entity.call(req).await?;
			let parts = stream_response_to_stdout(res).await?;
			let exit = match parts.status_to_exit_code() {
				Ok(()) => AppExit::Success,
				Err(code) => {
					error!("Command failed\nStatus code: {code}");
					AppExit::Error(code)
				}
			};
			entity.world().write_message(exit);
			Ok(())
		}),
	)
}

/// Streams a [`Response`] body to stdout chunk-by-chunk, returning
/// the response parts for exit-code inspection.
pub(crate) async fn stream_response_to_stdout(
	res: Response,
) -> Result<ResponseParts> {
	let (parts, mut body) = res.into_parts();
	while let Some(chunk) = body.next().await? {
		let chunk_str = String::from_utf8_lossy(&chunk);
		cross_log_noline!("{}", chunk_str);
	}
	Ok(parts)
}
