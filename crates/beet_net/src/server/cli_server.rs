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
/// This is how every beet binary boots: spawn it on the host, then trigger
/// [`StartServer::all`] (the empty filter matches the lone server). Being a
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
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_start_server);
}

/// Runs the one argv exchange when a [`StartServer`] passing `"cli"` lands.
fn on_start_server(ev: On<StartServer>, mut commands: Commands) {
	if !ev.passes("cli") {
		return;
	}
	// hold the process up for the duration of the async exchange with a
	// `KeepAliveGuard`; `run_and_exit` drops it when the exchange finishes, and a
	// despawn drops it automatically, so it cannot leak.
	// the boot params (from the `ServeOnLoad` verb, parsed from argv) reach the
	// exchange so a served site renders the requested route.
	let params = ev.params.clone();
	commands
		.entity(ev.event_target())
		.insert(KeepAliveGuard)
		.queue_async_local(move |entity| run_and_exit(entity, params));
}

/// The default content negotiation when `--accept` is unset.
fn default_accept() -> Vec<MediaType> {
	vec![
		MediaType::AnsiTerm,
		MediaType::Text,
		MediaType::Markdown,
		MediaType::Json,
	]
}

async fn run_and_exit(
	entity: AsyncEntity,
	params: MultiMap<SmolStr, SmolStr>,
) -> Result {
	// short-circuit when the entity has already been despawned, ie
	// [`TemplateStore::save_bundle`] briefly spawns a [`CliServer`] just to serialize
	// it. The despawn already dropped the `KeepAliveGuard`, so nothing leaks.
	if !entity.is_alive().await {
		return Ok(());
	}

	// A `path` boot param (eg `--server=cli --path=blog/post-1`) renders that site
	// route; absent it, parse the process argv (the binary's one-shot entrypoint).
	let req = match params.get("path") {
		Some(path) => {
			let accept = params
				.get("accept")
				.map(|item| MediaType::from_accepts(item))
				.unwrap_or_else(default_accept);
			Request::get(Url::parse(path.as_str()))
				.with_header::<header::Accept>(accept)
		}
		None => {
			let args = CliArgs::parse_env();
			let accept = args
				.params
				.get("accept")
				.map(|item| MediaType::from_accepts(item))
				.unwrap_or_else(default_accept);
			Request::from_cli_args(args).with_header::<header::Accept>(accept)
		}
	};

	let res = entity.exchange(req).await;
	let (parts, body) = res.into_parts();
	let exit_code = parts.status_to_exit_code();

	stream_body_to_stdout(body).await?;

	match exit_code {
		// success: drop our guard. If nothing else holds the process up the exit
		// system emits `AppExit::Success`; a sibling long-running server keeps it.
		Ok(()) => remove_keep_alive_guard(&entity).await,
		// failure: report the non-zero exit code directly so it reaches the process
		// exit, leaving our guard since the message ends the run anyway.
		Err(code) => {
			error!("Command failed\nStatus code: {code}");
			entity.world().write_message(AppExit::Error(code)).await;
		}
	}
	Ok(())
}

/// Drop the [`KeepAliveGuard`] a `cli` start inserted, so a finished exchange lets
/// the process exit unless another claim still holds it.
async fn remove_keep_alive_guard(entity: &AsyncEntity) {
	entity
		.with(|mut entity| {
			entity.remove::<KeepAliveGuard>();
		})
		.await
		.ok();
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
		app.world_mut()
			.spawn((
				CliServer,
				exchange_handler(|_| StatusCode::IM_A_TEAPOT.into_response()),
			))
			.trigger(StartServer::all);
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
