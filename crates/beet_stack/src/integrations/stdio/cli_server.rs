//! A single-shot CLI server that parses arguments from the environment,
//! dispatches them as a [`Request`], and streams the [`Response`] body
//! to stdout.
//!
//! Includes a [`markdown_render_tool`] so that cards render their
//! content to markdown for terminal output.
use crate::prelude::*;
use beet_core::prelude::*;

/// A single-shot CLI server [`Bundle`].
///
/// On spawn, parses the process CLI arguments into a [`Request`],
/// calls the owning entity's tool pipeline, streams the response body
/// to stdout, and writes an [`AppExit`] message so the app terminates
/// with the appropriate exit code.
///
/// Includes a [`markdown_render_tool`] for rendering card content
/// to markdown in the terminal.
///
/// Typically combined with a [`default_router`] and some child
/// cards/tools to build a CLI application:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
///     app.world_mut().spawn((
///         default_router(),
///         cli_server(),
///         children![
///             card("", || Paragraph::with_text("welcome!")),
///             increment(FieldRef::new("count")),
///             card("about", || Paragraph::with_text("about")),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn cli_server() -> impl Bundle {
	(
		OnSpawn::insert_child(markdown_render_tool()),
		OnSpawn::new(|entity| {
			entity.run_async(async |entity| -> Result {
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
			});
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
