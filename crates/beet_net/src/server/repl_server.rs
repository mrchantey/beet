//! A REPL server that reads lines from stdin in a loop, dispatching
//! each as a [`Request`] and streaming the [`Response`] body to stdout.
//!
//! Typically paired with a [`SceneToolRenderer`] so that scenes
//! render their content for terminal output.
//!
//! Uses a background thread for stdin reading so the async executor
//! is never blocked.
use super::cli_server::stream_body_to_stdout;
use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::prelude::*;
use beet_tool::prelude::*;

/// A REPL (read-eval-print loop) server [`Bundle`].
///
/// On spawn, parses process CLI arguments and dispatches them as an
/// initial [`Request`]. When no arguments are provided, the empty path
/// renders the root content. After the initial display, starts a
/// background thread that reads lines from stdin.
/// Each non-empty line is parsed as CLI-style arguments into a
/// [`Request`], dispatched through the owning entity's tool pipeline,
/// and the response body is streamed to stdout.
///
/// Typically paired with a [`SceneToolRenderer`] for rendering
/// scene content in the terminal, and a [`History`] component for
/// tracking the current path, enabling relative navigation via
/// `--navigate=<direction>`.
///
/// Typing `exit` or `quit` terminates the loop and writes
/// [`AppExit::Success`]. An EOF on stdin also exits cleanly.
///
/// Typically combined with a [`router`] and child tools
/// to build an interactive CLI application:
///
/// ```ignore
/// use beet_core::prelude::*;
/// use beet_router::prelude::*;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((MinimalPlugins, LogPlugin::default(), RouterAppPlugin));
///     app.world_mut().spawn((
///         SceneToolRenderer::default(),
///         router(),
///         ReplServer,
///         children![
///             scene_func("", || Name::new("welcome!")),
///             scene_func("about", || Name::new("about")),
///         ],
///     ));
///     async_ext::block_on(app.run_async());
/// }
/// ```

#[derive(Default, Component)]
#[component(on_add=on_add)]
pub struct ReplServer;

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).queue_async(repl_loop);
}

async fn repl_loop(entity: AsyncEntity) -> Result {
	let accept = vec![MediaType::AnsiTerm, MediaType::Markdown];
	// Dispatch CLI args as the initial request, rendering the
	// root content when no args are provided.
	call(
		&entity,
		Request::from_cli_args(CliArgs::parse_env())?
			.with_header::<header::Accept>(accept.clone()),
	)
	.await?;

	cross_log_noline!("> ");
	let stdin = stdin_lines();

	while let Ok(line) = stdin.recv().await {
		let trimmed = line.trim();
		if trimmed == "exit" || trimmed == "quit" {
			break;
		}

		call(
			&entity,
			Request::from_cli_str(trimmed)?
				.with_header::<header::Accept>(accept.clone()),
		)
		.await?;

		cross_log_noline!("> ");
	}

	entity.world().write_message(AppExit::Success);
	Ok(())
}

async fn call(entity: &AsyncEntity, request: Request) -> Result {
	// The repl server always prints help after rendering
	// unless its already a help call.
	let help_req = if !request.has_param("help") {
		Some(Request::from_parts(
			request.parts().clone().with_flag("help"),
			default(),
		))
	} else {
		None
	};

	let response: Response = entity.call(request).await?;
	let (parts, body) = response.into_parts();
	stream_body_to_stdout(body).await?;
	cross_log!("");
	if parts.status().is_err() {
		error!("command failed\nStatus: {}", parts.status());
	}
	if let Some(help_req) = help_req {
		cross_log!("");
		Box::pin(call(entity, help_req)).await?;
	}
	Ok(())
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

// nothing really to test, its all just stdio
