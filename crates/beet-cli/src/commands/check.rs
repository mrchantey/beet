use crate::prelude::*;
use beet::prelude::*;

/// Request params for the [`Check`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct CheckParams {
	/// Also write a JSON diagnostics manifest (the tags/classes/routes/style-props
	/// a future editor would autocomplete against) to this path, or `-` for stdout.
	manifest: Option<String>,
}

/// Validates a no-code BSX site without rendering it to disk: builds the site
/// world like [`ExportStatic`], then runs the render-diagnostics pass
/// over every static route, printing a summary and exiting NON-ZERO if any
/// error-level diagnostic fired.
///
/// This is the no-code "type-checker": it recovers the guarantees a markup site
/// loses without a compiler — an unknown `<Tag/>`, a broken internal `href`, an
/// unknown class — and gates CI on them. With `--manifest` it additionally exports
/// the *proactive* companion of those checks: a machine-readable
/// [`DiagnosticsManifest`] of what a future editor would validate and autocomplete.
///
/// ```sh
/// beet check examples/bsx_site                          # exit 0 if clean, non-zero on an error
/// beet check examples/bsx_site --manifest out.json      # also dump the manifest to a file
/// beet check examples/bsx_site --manifest=-             # ..or to stdout (then the summary)
/// ```
#[action(route = "check/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<CheckParams>())]
pub async fn Check(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let params = parts.params().parse_reflect::<CheckParams>()?;
	let SiteEntry { site_dir, entry } = resolve_site(&site_arg(parts)?)?;
	let root = build_site(&cx.caller, site_dir.clone(), entry).await?;
	let report = check_routes(&cx.world(), root).await?;

	// surface every diagnostic loudly through the log facade, then summarize.
	report.log();
	let summary = format!(
		"checked {} routes: {} error(s), {} warning(s)\n",
		report.checked.len(),
		report.error_count(),
		report.warn_count(),
	);

	// `--manifest <path|->`: export the editor-facing manifest of the loaded site
	// (built from the same world/RouteTree/RuleSet the checks read).
	if let Some(out) = params.manifest {
		write_manifest(&cx.world(), root, &out).await?;
	}

	// an error-level diagnostic fails the command (non-zero exit); a clean run (or
	// warnings only) succeeds.
	if report.has_errors() {
		Response::status_text(StatusCode::INTERNAL_SERVER_ERROR, summary).xok()
	} else {
		Response::ok_text(summary).xok()
	}
}

/// Build the site's [`DiagnosticsManifest`] and write it as pretty JSON to `out`
/// (a file path, or `-` for stdout). Runs in the loaded world so the registries,
/// `RuleSet` and `RouteTree` are all live.
async fn write_manifest(world: &AsyncWorld, root: Entity, out: &str) -> Result {
	let manifest = world
		.with(move |world: &mut World| build_diagnostics_manifest(world, root))
		.await?;
	// serialize through the codebase's `Value` JSON path rather than depending on
	// `serde_json` directly.
	let json = Value::from_serde(&manifest)?.to_string_pretty()?;
	// `-` streams the manifest to stdout (the program's actual output, never a log
	// prefix); any other value is a file path written via the cross-platform fs.
	if out == "-" {
		cross_log!("{json}");
	} else {
		fs_ext::write(out, &json)?;
		info!("wrote diagnostics manifest to {out}");
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	fn site_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	/// Render `req` through a host carrying only the [`Check`] route, returning the
	/// response (so a test can assert the status that drives the exit code).
	async fn run(req: Request) -> Response {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		)
			.into_world();
		let host = world.spawn((Router, children![Check])).id();
		world
			.entity_mut(host)
			.call::<Request, Response>(
				req.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap()
	}

	/// `beet check` over the clean demo site succeeds (a 2xx status → exit 0) and
	/// reports the routes it scanned.
	#[beet::test]
	async fn checks_clean_site() {
		let args = CliArgs::parse(&format!("check {}", site_path()));
		let response = run(Request::from_cli_args(args)).await;
		// a clean site is a success status, so the CLI exits 0.
		response.status().is_success().xpect_true();
		response
			.unwrap_str()
			.await
			.as_str()
			.xpect_contains("checked")
			.xpect_contains("0 error(s)");
	}
}
