//! A single-shot CLI server that parses arguments from the environment,
//! dispatches them as a [`Request`], and streams the [`Response`] body
//! to stdout.
use crate::prelude::*;
use beet_core::prelude::*;

/// A single-shot CLI server [`Bundle`].
///
/// On spawn, parses the process CLI arguments into a [`Request`],
/// calls the owning entity's tool pipeline, streams the response body
/// to stdout, and writes an [`AppExit`] message so the app terminates
/// with the appropriate exit code.
///
/// Typically combined with a [`default_interface`] and some child
/// cards/tools to build a CLI application. A [`Card`] with no
/// [`PathPartial`] matches the empty path, serving as root content:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
///     app.world_mut().spawn((
///         default_interface(),
///         cli_server(),
///         children![
///             (Card, children![Paragraph::with_text("welcome!")]),
///             increment(FieldRef::new("count")),
///             card("about"),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn cli_server() -> impl Bundle {
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
	})
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



#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn dispatches_help_request() {
		StackPlugin::world()
			.spawn((default_interface(), children![
				increment(FieldRef::new("count")),
				card("about"),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn dispatches_tool_request() {
		StackPlugin::world()
			.spawn((default_interface(), children![increment(FieldRef::new(
				"count"
			))]))
			.call::<Request, Response>(
				Request::from_cli_str("increment").unwrap(),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn not_found() {
		StackPlugin::world()
			.spawn((default_interface(), children![increment(FieldRef::new(
				"count"
			))]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NotFound);
	}

	#[beet_core::test]
	async fn renders_root_card_on_empty_args() {
		StackPlugin::world()
			.spawn((default_interface(), children![
				(Card, children![
					Heading1::with_text("My Server"),
					Paragraph::with_text("welcome!"),
				]),
				card("about"),
			]))
			.call::<Request, Response>(Request::from_cli_str("").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("My Server")
			.xpect_contains("welcome!");
	}

	#[beet_core::test]
	async fn scoped_help_for_subcommand() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((default_interface(), children![
				(card("counter"), children![increment(FieldRef::new(
					"count"
				)),]),
				card("about"),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("counter --help").unwrap(),
			)
			.await
			.unwrap();

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}
}
