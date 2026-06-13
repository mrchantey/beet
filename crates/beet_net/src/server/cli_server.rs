//! CLI-based server for running beet applications from the command line.
//!
//! This module provides [`CliServer`], which accepts command-line arguments
//! as a request and logs the response to stdout. Useful for CLI actions and
//! scripting.
//!
//! ## Accept Header
//!
//! Use `--accept` to specify preferred response media types:
//! ```sh
//! cargo run --example router -- --accept=text/html
//! cargo run --example router -- --accept=text/html,text/plain
//! ```
//! When omitted the default preference is `ansi-term, text, markdown, json`.
use crate::prelude::*;
use beet_core::prelude::*;

/// The entrypoint [`ServerBackend`]: accepts CLI arguments and environment
/// variables as a request, runs one exchange, streams the response body to
/// stdout, then exits (unless [`KeepAlive`] is set).
///
/// This is how every beet binary boots: spawning it pulls in the [`Server`]
/// orchestrator (via `#[require(Server)]`), which starts it through
/// [`CliServer::start`]. Being a one-shot, it cannot [`stop`](ServerBackend::stop)
/// (the default no-op).
///
/// Supports `--accept=<media types>` to override the default content negotiation,
/// for example `--accept=text/html,text/plain`.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Server)]
pub struct CliServer;

impl ServerBackend for CliServer {
	/// Parse argv into a request, run one exchange, stream the body to stdout,
	/// and exit unless [`KeepAlive`] is set. The entrypoint backend; it has no
	/// listener to stop, so it keeps the default no-op [`stop`](ServerBackend::stop).
	fn start(entity: AsyncEntity) -> MaybeSendBoxedFuture<'static, Result> {
		Box::pin(run_and_exit(entity))
	}
}

async fn run_and_exit(entity: AsyncEntity) -> Result {
	// short-circuit when the entity has already been despawned, ie
	// [`TemplateStore::save_bundle`] briefly spawns a [`CliServer`]
	// just to serialize it.
	if !entity.is_alive().await {
		return Ok(());
	}
	let args = CliArgs::parse_env();

	let accept = args
		.params
		.get("accept")
		.map(|item| MediaType::from_accepts(item))
		.unwrap_or_else(|| {
			vec![
				MediaType::AnsiTerm,
				MediaType::Text,
				MediaType::Markdown,
				MediaType::Json,
			]
		});

	let req =
		Request::from_cli_args(args).with_header::<header::Accept>(accept);

	let res = entity.exchange(req).await;
	let (parts, body) = res.into_parts();

	let exit = match parts.status_to_exit_code() {
		Ok(()) => AppExit::Success,
		Err(code) => {
			error!("Command failed\nStatus code: {code}");
			AppExit::Error(code)
		}
	};

	stream_body_to_stdout(body).await?;

	// a `--watch` command inserts `KeepAlive` so the schedule keeps running and
	// the file watcher can fire; otherwise the process exits after one command.
	let keep_alive = entity
		.world()
		.with(|world| world.contains_resource::<KeepAlive>())
		.await;
	if !keep_alive {
		entity.world().write_message(exit).await;
	}
	Ok(())
}

/// Streams a [`Response`] body to stdout chunk-by-chunk, returning
/// the response parts for exit-code inspection.
pub(crate) async fn stream_body_to_stdout(mut body: Body) -> Result {
	while let Some(chunk) = body.next().await? {
		let chunk_str = String::from_utf8_lossy(&chunk);
		cross_log_noline!("{}", chunk_str);
	}
	Ok(())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn cli_server_works() {
		App::new()
			.add_plugins((MinimalPlugins, ServerPlugin))
			.spawn((
				CliServer,
				exchange_handler(|_| StatusCode::IM_A_TEAPOT.into_response()),
			))
			.run_async()
			.await
			.xpect_eq(AppExit::Error(1.try_into().unwrap()));
	}

	#[beet_core::test]
	fn into_request_simple_path() {
		Request::from_cli_str("foo bar")
			.path_string()
			.xpect_eq("/foo/bar");
	}

	#[beet_core::test]
	fn into_request_with_query() {
		let req = Request::from_cli_str("api users --id=123");
		req.path_string().xpect_eq("/api/users");
		req.get_param("id").xpect_some();
	}

	#[beet_core::test]
	fn into_request_empty() {
		Request::from_cli_str("").path_string().xpect_eq("/");
	}
}
