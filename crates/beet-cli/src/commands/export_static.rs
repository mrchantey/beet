use crate::prelude::*;
use beet::prelude::*;

/// Statically exports a no-code BSX site: builds the site world like [`Serve`],
/// then renders every static route once and writes it under `<site>/dist`.
///
/// A one-shot generation, not a serve mode: the site root is the router (the
/// entry's root element is built into it), so it exports directly.
///
/// ```sh
/// beet export-static examples/bsx_site   # writes examples/bsx_site/dist
/// ```
#[action(route = "export-static/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ExportStatic(cx: ActionContext<Request>) -> Result<Response> {
	let site = site_arg(cx.input.request_parts())?;
	let SiteEntry { site_dir, entry } = resolve_site(&site)?;
	let root = build_site(&cx.caller, site_dir.clone(), entry).await?;
	let out = BlobStore::new(FsStore::new(site_dir.join("dist")));
	let written = export_static(&cx.world(), root, &out).await?;
	Response::ok_text(format!("exported {} routes to dist\n", written.len()))
		.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	fn site_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	/// Render `req` through a host carrying only the [`ExportStatic`] route.
	async fn run(req: Request) -> String {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		)
			.into_world();
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

	/// `export-static` statically exports the markup site to `<site>/dist`, the
	/// no-main.rs replacement for the example's old `export` route.
	///
	/// Built through [`Request::from_cli_args`], the way the CLI invokes it, so the
	/// absolute `<site>` positional round-trips as absolute (`*site` keeps its
	/// leading `/`); a stringified `format!("export-static/{abs}")` URL would drop it.
	#[beet::test]
	async fn exports_static_site() {
		let args = CliArgs::parse(&format!("export-static {}", site_path()));
		run(Request::from_cli_args(args))
			.await
			.as_str()
			.xpect_contains("exported")
			.xpect_contains("routes to dist");
		// the home route landed as the dist index, wrapped in the layout
		fs_ext::read_to_string(site_path().join("dist/index.html"))
			.unwrap()
			.as_str()
			.xpect_contains("<html lang=\"en\">")
			.xpect_contains("BSX Site");
	}
}
