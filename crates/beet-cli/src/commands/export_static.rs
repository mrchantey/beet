use crate::prelude::*;
use beet::prelude::*;

/// The default output directory, relative to the site dir, when `--out` is unset.
const DIST_DIR: &str = "dist";

/// Request params for the [`ExportStatic`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ExportStaticParams {
	/// Output directory for the rendered site (default `<site>/dist`).
	out: Option<String>,
}

/// Statically exports a no-code site: builds the site world (loads its entry
/// document), then renders every static route once and writes it to the output dir
/// (default `<site>/dist`, overridable with `--out`).
///
/// A one-shot generation, not a serve mode: the site root is the router (the
/// entry's root element is built into it), so it exports directly.
///
/// ```sh
/// beet export-static examples/bsx_site             # writes examples/bsx_site/dist
/// beet export-static examples/bsx_site --out=public # ..or a chosen output dir (cwd-relative)
/// ```
#[action(route = "export-static/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ExportStaticParams>())]
pub async fn ExportStatic(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let params = parts.params().parse_reflect::<ExportStaticParams>()?;
	let SiteEntry { site_dir, entry } = resolve_site(&site_arg(parts)?)?;
	let root =
		build_site(&cx.caller, parts.params(), site_dir.clone(), entry).await?;

	// validate before writing: the render-diagnostics pass gates CI, so a broken
	// no-code site (unknown tag, dead link) fails the export with a non-zero exit
	// rather than shipping a broken output.
	let report = check_routes(&cx.world(), root).await?;
	report.log();
	if report.has_errors() {
		return Response::status_text(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!(
				"export aborted: {} render error(s), see log\n",
				report.error_count()
			),
		)
		.xok();
	}

	// the output is a local write target, so an `fs` store: `--out` (absolute, or
	// relative to the cwd) overrides the default `<site>/dist`.
	let out_dir = match params.out {
		Some(out) => AbsPathBuf::new(out)?,
		None => site_dir.join(DIST_DIR),
	};
	let out = BlobStore::new(FsStore::new(out_dir.clone()));
	let written = export_static(&cx.world(), root, &out).await?;
	Response::ok_text(format!(
		"exported {} routes to {out_dir}\n",
		written.len()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	fn site_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	/// Run `export-static <args>` through a host carrying only the route, the way the
	/// CLI invokes it ([`Request::from_cli_args`], so an absolute `<site>` positional
	/// round-trips as absolute: `*site` keeps its leading `/`, which a stringified URL
	/// would drop). Returns the response text.
	async fn run(args: &str) -> String {
		let req =
			Request::from_cli_args(CliArgs::parse(&format!("export-static {args}")));
		let mut world = crate::commands::render_world();
		let host = world.spawn((Router, children![ExportStatic])).id();
		world
			.entity_mut(host)
			.call::<Request, Response>(
				req.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
	}

	/// `export-static` statically exports the markup site to its default `<site>/dist`,
	/// the no-main.rs replacement for the example's old `export` route.
	#[beet::test]
	async fn exports_static_site() {
		let dist = site_path().join("dist");
		run(&site_path().to_string())
			.await
			.as_str()
			.xpect_contains("exported")
			.xpect_contains(dist.to_string_lossy().as_ref());
		// the home route landed as the dist index, wrapped in the layout
		fs_ext::read_to_string(dist.join("index.html"))
			.unwrap()
			.as_str()
			.xpect_contains("<html lang=\"en\">")
			.xpect_contains("BSX Site");
		fs_ext::remove(&dist).ok();
	}

	/// `--out` redirects the export to a chosen directory rather than `<site>/dist`.
	#[beet::test]
	async fn export_out_override() {
		let out = TempDir::new().unwrap();
		run(&format!("{} --out={}", site_path(), out.path()))
			.await
			.as_str()
			.xpect_contains(out.path().to_string_lossy().as_ref());
		// the index rendered into the chosen dir
		out.path().join("index.html").exists().xpect_true();
	}
}
