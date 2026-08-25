use crate::prelude::*;
use beet::prelude::*;

/// Loads a no-code BSX entry and boots its servers, holding the process open: the
/// `beet serve <entry>` command.
///
/// `<entry>` is a directory (an entry document is discovered inside it) or the
/// entry file itself, so all of these resolve the same entry:
///
/// ```sh
/// beet serve site                       # the beet website (site/main.bsx)
/// beet serve site/main.bsx
/// beet serve examples/bsx_site          # a zero-code BSX site
/// beet serve examples/bsx_site/main.bsx
/// ```
///
/// `--server` selects which of the entry's declared servers start, defaulting to
/// `http`:
///
/// ```sh
/// beet serve site                       # http (the default)
/// beet serve site --server=tui          # a live terminal (like `beet present`)
/// beet serve site --server=http,ssh     # http plus a multi-tenant ssh terminal
/// ```
///
/// The entry is built disarmed ([`build_entry`] inserts [`DisableCallOnReady`]),
/// then booted explicitly: [`CallOnReady::call_recursive`] calls every declared
/// server under the loaded root (an explicit boot ignores the disarm), so the
/// loaded entry serves only when `serve` is invoked, never on its own load. A
/// parked server holds the await, so this handler never returns and the process
/// serves until interrupted.
#[action(route = "serve/*entry", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<EntryParams>())]
pub async fn Serve(cx: ActionContext<Request>) -> Result<Response> {
	let caller = cx.caller.clone();
	let request = cx.take();
	let parts = request.request_parts();
	// load the entry with boot suppressed, settling its `<RoutesDir>` discovery,
	// then boot the loaded root's servers explicitly: a parked server holds the
	// await, so this never returns and the process serves the entry.
	// `None`: a long-running server waits out a slow dependency rather than
	// failing the run, see `build_entry`.
	let root = build_entry(
		&caller,
		EntryParams::store(parts)?.as_ref(),
		&entry_arg(parts)?,
		None,
	)
	.await?;
	let root = caller.world().entity(root);
	let dispatches = root
		.get(|server: &CliServer| dispatches_commands(Some(server)))
		.await
		.unwrap_or_else(|_| dispatches_commands(None));
	CallOnReady::call_recursive(root, || entry_boot_request(parts, dispatches))
		.await?;
	Response::ok().xok()
}

/// Whether the loaded entry root dispatches commands, so its site is one route
/// among them rather than the root itself.
///
/// A dispatcher names its site with a positional route, so `serve` is forwarded
/// into it — the same shape the deployed unit uses (`app --store=.. serve`), so
/// `beet serve site` and the box run the identical path. An entry that IS the
/// server boots at its own home instead.
///
/// `always` is the discriminator: a pure dispatcher sets it, since it carries no
/// long-running servers and `--server` must not silence it, while an entry that
/// is its own server keeps a plain `CliServer` for one-shot renders beside its
/// transports.
fn dispatches_commands(root: Option<&CliServer>) -> bool {
	root.is_some_and(|server| server.always)
}

/// The boot request handed to the loaded entry: a fresh request carrying the
/// serve invocation's flags (`--server`, `--color-scheme`, `--port`, ...), with
/// the `serve/<entry>` command path and its `*entry` capture dropped so the
/// entry never treats the command path as a route.
///
/// The path is the entry's home (`/`) when the entry root IS the server, and the
/// positional `serve` route when it is a pure `CliServer` dispatcher (`always`)
/// whose site is one route among its commands. That `serve` path *addresses* the
/// command route; the servers it starts open at the url space their router
/// serves, not at that path (see [`OpeningRoute::from_parts`]).
///
/// `--server` defaults to `http`, so a bare `beet serve <entry>` brings up a web
/// server rather than every declared server: the `TuiServer` would seize the
/// terminal.
fn entry_boot_request(parts: &RequestParts, dispatches: bool) -> Request {
	let mut boot = match dispatches {
		true => RequestParts::get("serve"),
		false => RequestParts::get(Url::NONE),
	};
	for (key, values) in parts.params().iter_all() {
		// the greedy `*entry` capture is a serve-command concern, not an entry flag.
		if key.as_str() == "entry" {
			continue;
		}
		if values.is_empty() {
			boot.insert_flag(key.clone());
		} else {
			for value in values {
				boot.insert_param(key.clone(), value.clone());
			}
		}
	}
	if boot.get_params("server").is_none() {
		boot.insert_param("server", "http");
	}
	Request::from_parts(boot, default())
}

#[cfg(test)]
mod test {
	use super::*;

	/// A bare `beet serve <entry>` boots the entry at its home with
	/// `--server=http`, not the `serve/<entry>` command path: the `*entry` capture
	/// is dropped and the path is empty (root), so the entry's servers open their
	/// own home.
	#[beet::test]
	fn boot_request_defaults_to_http_at_home() {
		// mirror the routed request: the `*entry` capture plus no `--server`
		let mut parts = RequestParts::get("/serve/examples/bsx_site");
		parts.insert_param("entry", "examples/bsx_site");
		let boot = entry_boot_request(&parts, false);
		let parts = boot.request_parts();
		parts.get_param("server").xpect_eq(Some("http"));
		// the command path and its capture never reach the entry
		parts.get_param("entry").xpect_none();
		parts.path().is_empty().xpect_true();
	}

	/// An explicit `--server` wins over the default, and unrelated flags carry
	/// through to the entry (eg `--color-scheme`).
	#[beet::test]
	fn boot_request_preserves_explicit_flags() {
		let mut parts = RequestParts::get("/serve/site");
		parts.insert_param("entry", "site");
		parts.insert_param("server", "tui");
		parts.insert_param("color-scheme", "light");
		let boot = entry_boot_request(&parts, false);
		let parts = boot.request_parts();
		parts.get_param("server").xpect_eq(Some("tui"));
		parts.get_param("color-scheme").xpect_eq(Some("light"));
		parts.get_param("entry").xpect_none();
	}

	/// The `serve` path is forwarded only to a *pure* command dispatcher, the one
	/// shape whose site is a route among its commands.
	///
	/// Regression: the check was mere `CliServer` presence, so `beet serve
	/// examples/bsx_site --server=tui` booted that entry at `/serve`, a route it
	/// does not have, and its terminal opened on a "no route matched" page.
	#[beet::test]
	fn only_a_pure_dispatcher_takes_the_serve_path() {
		// the beet site: a dispatcher whose site hangs off its `serve` route
		dispatches_commands(Some(&CliServer { always: true })).xpect_true();
		// examples/bsx_site: the root IS the server, keeping a one-shot
		// `CliServer` beside its own transports
		dispatches_commands(Some(&CliServer::default())).xpect_false();
		// examples/thread/*: no dispatcher at all
		dispatches_commands(None).xpect_false();
	}

	/// An entry whose root is a pure `CliServer` dispatcher (the beet site) is
	/// booted by forwarding the positional `serve` route into it, the identical
	/// path the deployed unit takes (`app --store=.. --server=http,ssh serve`).
	#[beet::test]
	fn boot_request_forwards_serve_into_a_dispatcher() {
		let mut parts = RequestParts::get("/serve/site");
		parts.insert_param("entry", "site");
		parts.insert_param("server", "http,ssh");
		let boot = entry_boot_request(&parts, true);
		let parts = boot.request_parts();
		parts.path_string().xpect_eq("/serve");
		parts.get_param("server").xpect_eq(Some("http,ssh"));
		parts.get_param("entry").xpect_none();
	}
}
