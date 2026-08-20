//! Boot a server host declared as a child, from a dispatched route.

use crate::prelude::*;
use beet_core::prelude::*;

/// Boot every server host declared under this entity, parking until they stop:
/// the action a `serve` route hangs off when the entry root is a dispatcher
/// rather than the server itself.
///
/// An entry whose root is a [`CliServer`] selects what to run with a positional
/// route, so the served site is one route among the entry's commands:
///
/// ```bsx
/// <Route path="serve" {BootHost}>
///     <StartOnLoad {(HttpServer, SshTuiServer)}>
///         <Router>..</Router>
///     </StartOnLoad>
/// </Route>
/// ```
///
/// Requires [`DisableCallOnLoad`], since the host would otherwise ALSO fire its
/// own load call when the entry builds and boot twice: the point of putting a
/// site behind a route is that it starts when the route is dispatched and not
/// before. [`CallOnLoad::call_recursive`] ignores that opt-out by design, so the
/// explicit boot still runs.
///
/// The hosted servers park, so this never resolves and the process serves until
/// interrupted.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DisableCallOnLoad)]
pub async fn BootHost(cx: ActionContext<Request>) -> Result<Response> {
	let caller = cx.caller.clone();
	let request = cx.take();
	// the hosts open their own home rather than treating the dispatching route
	// path as a route, but keep the invocation's flags (`--server`, `--port`, ..).
	let mut boot = RequestParts::get(Url::NONE);
	for (key, values) in request.request_parts().params().iter_all() {
		match values.is_empty() {
			true => boot.insert_flag(key.clone()),
			false => values.iter().for_each(|value| {
				boot.insert_param(key.clone(), value.clone())
			}),
		}
	}
	CallOnLoad::call_recursive(caller, move || {
		Request::from_parts(boot.clone(), default())
	})
	.await?;
	Response::ok().xok()
}
