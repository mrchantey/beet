use crate::prelude::*;
use beet::prelude::*;

/// The default output directory, relative to the entry dir, when `--out` is unset.
const DIST_DIR: &str = "dist";

/// Statically exports a no-code entry: builds the entry world (loads its entry
/// document), then renders every static route once and writes it to the output dir
/// (default `<entry>/dist`, overridable with `--out`).
///
/// A one-shot generation, not a serve mode: the entry root is the router (the
/// entry's root element is built into it), so it exports directly.
///
/// ```sh
/// beet export-static examples/beetmania             # writes examples/beetmania/dist
/// beet export-static examples/beetmania --out=public # ..or a chosen output dir (cwd-relative)
/// ```
#[action(route = "export-static/*entry")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ExportStatic(
	/// Output directory for the rendered entry (default `<entry>/dist`).
	#[param]
	out: Option<SmolPath>,
	entry_params: Params<EntryParams>,
	cx: ActionContext<RequestParts>,
) -> Result<Response> {
	let entry_path = entry_arg(&cx.input)?;
	let root = entry_params.load(&cx.caller, &entry_path).await?;

	// validate before writing: the render-diagnostics pass gates CI, so a broken
	// no-code entry (unknown tag, dead link) fails the export with a non-zero exit
	// rather than shipping a broken output.
	let report = CheckReport::check_routes(&cx.world(), root).await?;
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
	// relative to the cwd) overrides the default `<entry>/dist` (the entry's own
	// dir, not a `<PackageRoot>`-widened root).
	let out_dir = match out {
		Some(out) => AbsPath::new(out)?,
		None => entry_dir(&entry_path)?.join(DIST_DIR),
	};
	let out = BlobStore::new(FsStore::new(out_dir.clone()));
	let written = StaticExport::export(&cx.world(), root, &out).await?;
	Response::ok_text(format!(
		"exported {} routes to {out_dir}\n",
		written.len()
	))
	.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// Run `export-static <args>` through a host carrying only the route, the way the
	/// CLI invokes it ([`Request::from_cli_args`], so an absolute `<entry>` positional
	/// round-trips as absolute: `*entry` keeps its leading `/`, which a stringified URL
	/// would drop). Returns the response text.
	async fn run(args: &str) -> String {
		let req = Request::from_cli_args(CliArgs::parse(&format!(
			"export-static {args}"
		)));
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
		let site = crate::commands::clean_site();
		let dist = site.path().join("dist");
		run(&site.path().to_string())
			.await
			.as_str()
			.xpect_contains("exported")
			.xpect_contains(dist.as_str());
		// the home route landed as the dist index, wrapped in the layout
		fs_ext::read_to_string(dist.join("index.html"))
			.unwrap()
			.as_str()
			.xpect_contains("<html lang=\"en\">")
			.xpect_contains("<title>Home</title>");
	}

	/// `--out` redirects the export to a chosen directory rather than `<site>/dist`.
	#[beet::test]
	async fn export_out_override() {
		let site = crate::commands::clean_site();
		let out = TempDir::new().unwrap();
		run(&format!("{} --out={}", site.path(), out.path()))
			.await
			.as_str()
			.xpect_contains(out.path().as_str());
		// the index rendered into the chosen dir
		fs_ext::exists(out.path().join("index.html"))
			.unwrap()
			.xpect_true();
	}
}
