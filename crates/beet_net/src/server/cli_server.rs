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

/// The entrypoint server: adds a one-shot facet to its entity's [`RunningSet`]
/// that routes the start request through the host's dispatch (see
/// [`exchange`](AsyncExchangeExt::exchange)) and resolves the parked call with
/// the response.
///
/// This is how every beet binary boots by default: it owns the entry root with
/// the dispatch host as its child, and the load path (no `--server`, or
/// `--server=cli`) reaches it.
///
/// ```bsx
/// <CliServer always=true {CallOnReady}>
///     <Router>..</Router>
/// </CliServer>
/// ```
///
/// Being a one-shot it resolves the call rather than holding it, so the process
/// exits once [`CallOnReady::call`] has streamed its response. Resolving removes
/// the entity's `Running<Response>`, which signals any co-resident facet: a bare
/// start of an entity carrying both a `CliServer` and an [`HttpServer`]
/// dispatches once and exits.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(CliServer::add_facet))]
pub struct CliServer {
	/// Dispatch on every boot, ignoring `--server`.
	///
	/// A site's default `CliServer` only acts when `--server` selects `cli`, so
	/// `--server=http` serves http rather than streaming once to stdout. A
	/// command-dispatcher root sets this, because `--server` names the transports
	/// of whichever route it dispatches INTO, never its own: `beet --main=site
	/// serve --server=http` has to reach the `serve` route before the `http`
	/// selection means anything, so the dispatch itself cannot be subject to it.
	pub always: bool,
}

impl CliServer {
	/// This server's [`RunningSet`] facet: dispatch the start request, resolve the
	/// parked call with the response, and complete. There is no listener to close,
	/// so it holds nothing open across the shutdown signal.
	fn add_facet(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		// an `always` dispatcher (the workspace command entry) acts on every
		// start; otherwise `--server` decides, defaulting to acting.
		let always = self.always;
		move |entity: &mut EntityCommands| {
			RunningSet::<Request, Response>::add(
				entity,
				"cli",
				move |request: &Request| {
					always
						|| RunningSetFilter::selects(
							request.params(),
							"cli",
							true,
						)
				},
				|entity, request, _shutdown| {
					// the future owns its dispatch, so nothing borrows the input
					// past this call; a start request is argv-shaped and carries no
					// body, so cloning the parts is the whole copy.
					let dispatch = Request::from_parts(
						request.request_parts().clone(),
						default(),
					);
					Box::pin(route_and_end(entity, dispatch))
				},
			);
		}
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
/// with the response so [`CallOnReady::call`] streams it and exits.
///
/// Awaited inline by the facet rather than detached: the driver polls every
/// selected facet concurrently, so a dispatched route that parks (a `serve`
/// command) holds nothing else up.
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
