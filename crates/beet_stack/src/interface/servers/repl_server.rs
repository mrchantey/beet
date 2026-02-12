//! A REPL server that reads lines from stdin in a loop, dispatching
//! each as a [`Request`] and streaming the [`Response`] body to stdout.
//!
//! Uses a background thread for stdin reading so the async executor
//! is never blocked.
use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::prelude::*;

/// A REPL (read-eval-print loop) server [`Bundle`].
///
/// On spawn, parses process CLI arguments and dispatches them as an
/// initial [`Request`]. When no arguments are provided, the empty path
/// renders the root card. After the initial display, starts a
/// background thread that reads lines from stdin.
/// Each non-empty line is parsed as CLI-style arguments into a
/// [`Request`], dispatched through the owning entity's tool pipeline,
/// and the response body is streamed to stdout.
///
/// Includes a [`History`] component for tracking the current path,
/// enabling relative navigation via `--navigate=<direction>`.
///
/// Typing `exit` or `quit` terminates the loop and writes
/// [`AppExit::Success`]. An EOF on stdin also exits cleanly.
///
/// Typically combined with a [`markdown_interface`] and child tools
/// to build an interactive CLI application:
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_stack::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
///     app.world_mut().spawn((
///         Card,
///         markdown_interface(),
///         repl_server(),
///         children![
///             increment(FieldRef::new("count")),
///             card("about"),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```
pub fn repl_server() -> impl Bundle {
	(
		History::default(),
		OnSpawn::new(|entity| {
			entity.run_async(async |entity| -> Result {
				// Dispatch CLI args as the initial request, rendering the
				// root card when no args are provided.
				let initial_req = Request::from_cli_args(CliArgs::parse_env())?;
				let initial_res: Response = entity.call(initial_req).await?;
				let parts = stream_response_to_stdout(initial_res).await?;
				cross_log!("");
				if let Err(code) = parts.status_to_exit_code() {
					error!("Initial command failed\nStatus code: {code}");
				}

				cross_log_noline!("> ");
				let stdin = stdin_lines();

				while let Ok(line) = stdin.recv().await {
					let trimmed = line.trim();
					if trimmed.is_empty() {
						cross_log_noline!("> ");
						continue;
					}
					if trimmed == "exit" || trimmed == "quit" {
						break;
					}

					let req = Request::from_cli_str(trimmed)?;
					let res: Response = entity.call(req).await?;
					let parts = stream_response_to_stdout(res).await?;
					cross_log!("");

					if let Err(code) = parts.status_to_exit_code() {
						error!("Command failed\nStatus code: {code}");
					}

					cross_log_noline!("> ");
				}

				entity.world().write_message(AppExit::Success);
				Ok(())
			});
		}),
	)
}

fn stdin_lines() -> async_channel::Receiver<String> {
	let (tx, rx) = async_channel::unbounded::<String>();

	// Background thread reads stdin without blocking the executor
	std::thread::spawn(move || {
		let stdin = std::io::stdin();
		loop {
			let mut line = String::new();
			match stdin.read_line(&mut line) {
				Ok(0) => break, // EOF
				Ok(_) => {
					if tx.send_blocking(line).is_err() {
						break;
					}
				}
				Err(_) => break,
			}
		}
	});
	rx
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn dispatches_tool_request() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((Card, markdown_interface(), children![increment(
				FieldRef::new("count")
			)]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("increment").unwrap(),
			)
			.await
			.unwrap();

		let parts = stream_response_to_stdout(res).await.unwrap();
		parts.status().xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn help_flag_returns_route_list() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((Card, markdown_interface(), children![
				increment(FieldRef::new("count")),
				card("about"),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await;

		body.contains("Available routes").xpect_true();
	}
}
