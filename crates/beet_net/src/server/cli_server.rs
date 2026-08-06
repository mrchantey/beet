//! The one-shot CLI server: routes the boot request and resolves the boot call.
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
use beet_action::prelude::*;
use beet_core::prelude::*;

/// The entrypoint server: observes the boot [`StartRunning<Request>`], routes the
/// request through its host's dispatch (see [`exchange`](AsyncExchangeExt::exchange)),
/// then either resolves the boot call or streams the response and exits itself,
/// whichever the boot path needs.
///
/// Two boot paths land here. The load path ([`CallOnLoad`], required) fires
/// `StartRunning<Request>` behind a `Running<Response>` keep-alive, so `CliServer`
/// resolves it with an [`EndRun`] and [`CallOnLoad::call`] streams the response. A
/// direct `trigger(StartRunning::from_cli)` has no `Running`, so `CliServer` streams
/// the response and writes the [`AppExit`] itself.
///
/// This is how every beet binary boots by default: it owns the entry root with the
/// dispatch host as its child (`<CliServer><Router>..</Router></CliServer>`), and the
/// load path (no `--server`, or `--server=cli`) reaches it. Being a one-shot, it
/// resolves the call rather than parking, so the process exits once its response is
/// streamed.
///
/// Supports `--accept=<media types>` to override the default content negotiation,
/// for example `--accept=text/html,text/plain`.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(StartOnLoad)]
#[component(on_add = hook_ext::observe(on_action_in))]
pub struct CliServer {
	/// Dispatch on every boot, ignoring `--server`.
	///
	/// A site's default `CliServer` only acts when `--server` selects `cli`, so
	/// `--server=http` serves http rather than streaming once to stdout. The
	/// workspace command entry sets this: it carries no long-running servers, and a
	/// `--server` on a `beet serve <entry>` invocation selects the *entry's* servers,
	/// so the command dispatch itself must still run.
	pub always: bool,
}

/// On the boot fan-out, route the request and resolve the boot call when this server
/// should act: an `always` dispatcher (the workspace command entry) on every boot,
/// otherwise only when `--server` selects `cli`. The selection check reads the boot
/// (without consuming it); the take is deferred into the task, so a co-observer's
/// read never races it.
fn on_action_in(
	ev: On<StartRunning<Request>>,
	servers: Query<&CliServer>,
	mut commands: Commands,
) -> Result {
	let always = servers.get(ev.entity).is_ok_and(|server| server.always);
	// default-boots (the shared default) unless `--server` names a different set;
	// `always` additionally boots even when another server is named.
	if !always
		&& !ev.with(|request| Request::selects_server(request, "cli", true))?
	{
		return Ok(());
	}
	let start = ev.clone();
	commands
		.entity(ev.entity)
		// flags the boot as served, so `exit_if_no_server` lets it run
		.insert(ServerBooted)
		.queue_async_local(move |server| route_and_end(server, start));
	Ok(())
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

/// Route the request through the host's dispatch, then hand the response to
/// whichever boot path called us: resolve the `Running` keep-alive if the load
/// path set one (it streams and exits), otherwise stream and exit here ourselves.
async fn route_and_end(
	server: AsyncEntity,
	start: StartRunning<Request>,
) -> Result {
	let request = start.take()?;
	// `--accept` may arrive as several params (CliArgs splits comma lists), so
	// gather every value's media types.
	let accept = request
		.get_params("accept")
		.map(|values| {
			values
				.iter()
				.flat_map(|value| MediaType::from_accepts(value))
				.collect::<Vec<_>>()
		})
		.unwrap_or_else(default_accept);
	// a server holds no dispatch of its own: hop down to the router child.
	let response = server
		.exchange_child(request.with_header::<header::Accept>(accept))
		.await;
	// the load path parks on a Running; resolve it so `CallOnLoad::call` streams.
	// a direct `trigger(StartRunning::from_cli)` has none, so stream and exit ourselves.
	if server
		.with(|entity| entity.contains::<Running<Response>>())
		.await?
	{
		server.queue(EndRun(response)).await??;
	} else {
		stream_and_exit(&server, response).await?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

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
