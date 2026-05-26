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

/// A server that accepts CLI arguments and environment variables as a request,
/// logging the response body to stdout.
///
/// Supports `--accept=<media types>` to override the default content negotiation,
/// for example `--accept=text/html,text/plain`.
///
/// The `run_and_exit` task is launched on the next schedule tick via the
/// [`launch_cli_servers`] system. Deferring to a system (instead of an
/// `on_add` hook) avoids spawning a transient task when [`WorldSerdeStore`]
/// briefly spawns a [`CliServer`] just to serialize it.
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct CliServer;

/// Startup-style system that launches [`run_and_exit`] on each newly
/// added [`CliServer`] entity. Registered by [`ServerPlugin`].
pub fn launch_cli_servers(
	mut commands: Commands,
	query: Populated<Entity, Added<CliServer>>,
) {
	for entity in query.iter() {
		commands.entity(entity).queue_async(run_and_exit);
	}
}

async fn run_and_exit(entity: AsyncEntity) -> Result {
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
		Request::from_cli_args(args)?.with_header::<header::Accept>(accept);

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

	entity.world().write_message(exit);
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
			.spawn_then((
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
			.unwrap()
			.path_string()
			.xpect_eq("/foo/bar");
	}

	#[beet_core::test]
	fn into_request_with_query() {
		let req = Request::from_cli_str("api users --id=123").unwrap();
		req.path_string().xpect_eq("/api/users");
		req.get_param("id").xpect_some();
	}

	#[beet_core::test]
	fn into_request_empty() {
		Request::from_cli_str("")
			.unwrap()
			.path_string()
			.xpect_eq("/");
	}
}
