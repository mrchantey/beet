//! The one-shot CLI server: routes the boot request and resolves the parked call.
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

/// The entrypoint server: contributes a one-shot start to its entity's
/// [`RunningSet`] that routes the boot request through the host's dispatch (see
/// [`exchange`](AsyncExchangeExt::exchange)) and resolves the parked call with
/// the response.
///
/// This is how every beet binary boots by default: it owns the entry root with
/// the dispatch host as its child, and the load path (no `--server`, or
/// `--server=cli`) reaches it.
///
/// ```bsx
/// <CliServer always=true {CallOnLoad}>
///     <Router>..</Router>
/// </CliServer>
/// ```
///
/// Being a one-shot it resolves the call rather than parking, so the process
/// exits once [`CallOnLoad::call`] has streamed its response. The dispatch runs
/// detached, so a co-resident long-running server still starts and a dispatched
/// route that parks (a `serve` command) never holds the walk up.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::entity_hook(CliServer::contribute))]
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

impl CliServer {
	/// This server's [`RunningSet`] contribution: dispatch the boot request on
	/// start, with no listener to close on stop.
	fn contribute(entity: &mut EntityCommands) {
		RunningSet::<Request, Response>::contribute(
			entity,
			Self::start(),
			None,
		);
	}

	/// The start entry: dispatch when this server should act, detached so a
	/// dispatched route that parks never holds the walk up.
	///
	/// The request threads on to the next entry, so the dispatch takes a copy of
	/// its parts; a boot request is argv-shaped and carries no body.
	fn start() -> Action<Request, StartOutcome<Request>> {
		Action::new_async_local(|cx: ActionContext<Request>| async move {
			let entity = cx.caller;
			let request = cx.input;
			// an `always` dispatcher (the workspace command entry) acts on every
			// boot; otherwise `--server` decides, defaulting to acting.
			let acts = entity.get(|server: &CliServer| server.always).await?
				|| ServerFilter::selects(request.params(), "cli", true);
			if !acts {
				return StartOutcome::Declined(request).xok();
			}
			let dispatch =
				Request::from_parts(request.request_parts().clone(), default());
			entity
				.run_async_local(move |entity| route_and_end(entity, dispatch))
				.await?;
			StartOutcome::Started(request).xok()
		})
	}
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

/// Route the request through the host's dispatch, then resolve the parked call
/// with the response so [`CallOnLoad::call`] streams it and exits.
async fn route_and_end(server: AsyncEntity, request: Request) -> Result {
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
	server.queue(EndRun(response)).await?
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
