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

/// The entrypoint server: observes the boot [`ActionIn<Request>`], routes the
/// request through the host's [`RouteAction`], and resolves the boot call with an
/// [`EndRun`] so [`bootstrap`] streams the response and exits.
///
/// This is how every beet binary boots by default: spread it on the entry root,
/// and the boot fan-out (no `--server`, or `--server=cli`) reaches it. Being a
/// one-shot, it resolves the call rather than parking, so the process exits once
/// its response is streamed.
///
/// Supports `--accept=<media types>` to override the default content negotiation,
/// for example `--accept=text/html,text/plain`.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct CliServer;

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_action_in);
}

/// On the boot fan-out, if `--server` selects `cli`, route the request and resolve
/// the boot call. The selection check reads the request (without consuming it);
/// the take is deferred into the task, so a co-observer's read never races it.
fn on_action_in(ev: On<ActionIn<Request>>, mut commands: Commands) -> Result {
	if !ev.with(|req| request_selects_server(req, "cli"))? {
		return Ok(());
	}
	let action_in = ev.clone();
	commands
		.entity(ev.entity)
		.queue_async_local(move |host| route_and_end(host, action_in));
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

/// Route the request through the host and resolve the boot call with the response,
/// which [`bootstrap`] then streams to stdout.
async fn route_and_end(host: AsyncEntity, action_in: ActionIn<Request>) -> Result {
	let request = action_in.take()?;
	let accept = request
		.get_param("accept")
		.map(MediaType::from_accepts)
		.unwrap_or_else(default_accept);
	let response =
		host.route(request.with_header::<header::Accept>(accept)).await;
	host.queue(EndRun(response)).await??;
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
