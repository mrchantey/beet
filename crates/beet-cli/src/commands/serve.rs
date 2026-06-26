use crate::prelude::*;
use beet::prelude::*;

/// Loads a no-code BSX site and boots its servers, holding the process open: the
/// `beet serve <site>` command.
///
/// `<site>` is a site directory (an entry document is discovered inside it) or the
/// entry file itself, so all of these resolve the same site:
///
/// ```sh
/// beet serve site                       # the beet website (site/main.bsx)
/// beet serve site/main.bsx
/// beet serve examples/bsx_site          # a zero-code BSX site
/// beet serve examples/bsx_site/main.bsx
/// ```
///
/// `--server` selects which of the site's declared servers start, defaulting to
/// `http`:
///
/// ```sh
/// beet serve site                       # http (the default)
/// beet serve site --server=tui          # a live terminal (like `beet present`)
/// beet serve site --server=http,ssh     # http plus a multi-tenant ssh terminal
/// ```
///
/// The site entry is built with boot suppressed ([`build_site`] adds
/// [`DisableBootOnLoad`]), then booted explicitly through its boot slot — a direct
/// `entity(root).call::<Boot, Response>` on the loaded root — so the workspace entry
/// serves only when `serve` is invoked, never on an always-on `BootOnLoad`. The boot
/// call parks on the servers' `Running` keep-alive, so this handler never returns and
/// the process serves until interrupted.
#[action(route = "serve/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Serve(cx: ActionContext<Request>) -> Result<Response> {
	let caller = cx.caller.clone();
	let request = cx.take();
	let parts = request.request_parts();
	let SiteEntry { site_dir, entry } = resolve_site(&site_arg(parts)?)?;
	// load the site with boot suppressed, settling its `<RoutesDir>` discovery, then
	// boot the loaded root explicitly: the servers' boot slot parks on a `Running`
	// keep-alive, so this never returns and the process serves the site.
	let root = build_site(&caller, parts.params(), site_dir, entry).await?;
	// boot the site at its own home with the serve flags (see `site_boot_request`),
	// not the `serve/<site>` command request the dev router routed here.
	let boot = site_boot_request(parts);
	caller
		.world()
		.entity(root)
		.call::<Boot, Response>(Boot(boot))
		.await
}

/// The boot request handed to the loaded site: a fresh request at the site's home
/// (`/`) carrying the serve invocation's flags (`--server`, `--color-scheme`,
/// `--port`, ...), with the `serve/<site>` command path and its `*site` capture
/// dropped so the site's servers open their own home rather than treating the
/// command path as a route.
///
/// `--server` defaults to `http`, so a bare `beet serve <site>` brings up a web
/// server rather than every declared server: a one-shot `CliServer` would resolve
/// the boot and exit the process, and the `TuiServer` would seize the terminal.
fn site_boot_request(parts: &RequestParts) -> Request {
	let mut boot = RequestParts::get(Url::NONE);
	for (key, values) in parts.params().iter_all() {
		// the greedy `*site` capture is a serve-command concern, not a site flag.
		if key.as_str() == "site" {
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

	/// A bare `beet serve <site>` boots the site at its home with `--server=http`,
	/// not the `serve/<site>` command path: the `*site` capture is dropped and the
	/// path is empty (root), so the site's servers open their own home.
	#[beet::test]
	fn boot_request_defaults_to_http_at_home() {
		// mirror the routed request: the `*site` capture plus no `--server`
		let mut parts = RequestParts::get("/serve/examples/bsx_site");
		parts.insert_param("site", "examples/bsx_site");
		let boot = site_boot_request(&parts);
		let parts = boot.request_parts();
		parts.get_param("server").xpect_eq(Some("http"));
		// the command path and its capture never reach the site
		parts.get_param("site").xpect_none();
		parts.path().is_empty().xpect_true();
	}

	/// An explicit `--server` wins over the default, and unrelated flags carry
	/// through to the site (eg `--color-scheme`).
	#[beet::test]
	fn boot_request_preserves_explicit_flags() {
		let mut parts = RequestParts::get("/serve/site");
		parts.insert_param("site", "site");
		parts.insert_param("server", "tui");
		parts.insert_param("color-scheme", "light");
		let boot = site_boot_request(&parts);
		let parts = boot.request_parts();
		parts.get_param("server").xpect_eq(Some("tui"));
		parts.get_param("color-scheme").xpect_eq(Some("light"));
		parts.get_param("site").xpect_none();
	}
}
