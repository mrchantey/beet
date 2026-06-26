use crate::prelude::*;
use beet::prelude::*;

/// Loads a no-code BSX site and boots its servers, holding the process open: the
/// `beet serve <site>` command.
///
/// The site entry is built with boot suppressed ([`build_site`] adds
/// [`DisableBootOnLoad`]), then booted explicitly through its boot slot — a direct
/// `entity(root).call::<Boot, Response>` on the loaded root — so the workspace entry
/// serves only when `serve` is invoked, never on an always-on `BootOnLoad`. The boot
/// call parks on the servers' `Running` keep-alive, so this handler never returns and
/// the process serves until interrupted.
///
/// ```sh
/// beet serve examples/bsx_site --server=http      # serve the site over http
/// ```
#[action(route = "serve/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Serve(cx: ActionContext<Request>) -> Result<Response> {
	let caller = cx.caller.clone();
	let request = cx.take();
	let site = site_arg(request.request_parts())?;
	let SiteEntry { site_dir, entry } = resolve_site(&site)?;
	// load the site with boot suppressed, settling its `<RoutesDir>` discovery, then
	// boot the loaded root explicitly: the servers' boot slot parks on a `Running`
	// keep-alive, so this never returns and the process serves the site. The
	// `--server` selector rides the boot request to pick which servers start.
	let root =
		build_site(&caller, request.request_parts().params(), site_dir, entry)
			.await?;
	caller
		.world()
		.entity(root)
		.call::<Boot, Response>(Boot(request))
		.await
}
