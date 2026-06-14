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

/// The entrypoint server: on a [`StartServer`] whose filter passes `"cli"`, it
/// parses argv and environment into a request, runs one exchange, streams the
/// response body to stdout, then exits (unless [`KeepAlive`] is set).
///
/// This is how every beet binary boots: spawn it on the host alongside
/// [`bootstrap_cli`], which fires the `cli`-filtered [`StartServer`]. Being a
/// one-shot, [`StopServer`] is a no-op for it.
///
/// Supports `--accept=<media types>` to override the default content negotiation,
/// for example `--accept=text/html,text/plain`.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct CliServer;

/// Registers the [`StartServer`] observer on the host, so the one-shot exchange
/// runs when a start event whose filter passes `"cli"` lands on it.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_start_server);
}

/// Runs the one argv exchange when a [`StartServer`] passing `"cli"` lands.
fn on_start_server(ev: On<StartServer>, mut commands: Commands) {
	if !ev.passes("cli") {
		return;
	}
	commands.entity(ev.event_target()).queue_async_local(run_and_exit);
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

	// a long-running server (or a `--watch` command) inserts `KeepAlive` so the
	// schedule keeps running; otherwise the process exits after one command.
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
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		app.world_mut().spawn((
			CliServer,
			bootstrap_cli(),
			exchange_handler(|_| StatusCode::IM_A_TEAPOT.into_response()),
		));
		app.run_async()
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
