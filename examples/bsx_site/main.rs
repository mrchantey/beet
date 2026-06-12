//! # BSX site example
//!
//! A whole site declared in markup, no codegen: `main.bsx` is the entrypoint,
//! `routes/` is the content (markdown + BSX pages, discovered at spawn time),
//! and `templates/` holds the site's own BSX templates (the `Layout` document,
//! widgets). See `README.md` for the architecture and feature-parity notes.
//!
//! This binary is the thin generic host the `beet run <site-dir>` command
//! packages up (see beet-cli's `run_site.rs`): plugins + package config, then
//! spawn the entry file and layer the host concerns (server backend, dev
//! `export` route) onto it.
//!
//! ## Running
//!
//! ```sh
//! alias site='cargo run --example bsx_site --features "http_server,client_io,json,markdown,style,template,fs" --'
//!
//! # CLI mode: render the home route, or a named route
//! site
//! site docs/getting-started
//!
//! # HTTP mode, optionally live-reloading on site edits
//! site --server=http
//! site --server=http --watch
//!
//! # static export to examples/bsx_site/dist
//! site export
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin,
			ServerPlugin,
			material::MaterialStylePlugin::default(),
		))
		// compile-time package metadata only: the site's title and description
		// are declared in `main.bsx` via the `<PackageConfig/>` resource tag
		.insert_resource(pkg_config!())
		.add_systems(Startup, setup)
		.run()
}

/// Register the site's templates, spawn `main.bsx` as the app root, then layer
/// the host concerns onto it: the server backend and the dev `export` route.
fn setup(world: &mut World) -> Result {
	let site_dir = AbsPathBuf::new_workspace_rel("examples/bsx_site")?;
	// `<path::to::X>` tags resolve to the site's own templates
	world.register_bsx_templates(site_dir.join("templates"))?;
	// `<RoutesDir src=".."/>` resolves against the site root
	world.insert_resource(SiteRoot(site_dir.clone()));
	let root = BsxTemplate::load_entry(world, site_dir.join("main.bsx"))?
		.spawn(world)?;
	world.entity_mut(root).insert(server_from_cli()?);
	world.spawn((ChildOf(root), exchange_route("export", Export)));
	// the default app routes `default_router` wires for codegen hosts; the
	// `<PackageConfig/>` declared in `main.bsx` backs both
	world.spawn((ChildOf(root), app_info()));
	world.spawn((ChildOf(root), analytics_handler()));
	// dev mode: watch the site dir, respawning routes and live-reloading
	// browsers via the `<LiveReloadScript/>` in the layout head
	if CliArgs::parse_env().params.contains_key("watch") {
		world.spawn(LiveReload::new(site_dir));
	}
	Ok(())
}

/// Selects the server backend from `--server`, defaulting to `cli`.
fn server_from_cli() -> Result<OnSpawn> {
	match CliArgs::parse_env()
		.params
		.get("server")
		.map(|server| server.to_lowercase())
		.as_deref()
	{
		None | Some("cli") => CliServer::default().any_bundle().xok(),
		Some("http") => HttpServer::default().any_bundle().xok(),
		Some(other) => {
			bevybail!("invalid --server '{other}', expected 'cli' or 'http'")
		}
	}
}

/// Statically exports every static route to `examples/bsx_site/dist`.
#[action]
#[derive(Component)]
async fn Export(cx: ActionContext) -> Result<String> {
	let caller = cx.caller.clone();
	let world = cx.world();
	let router = caller
		.with_state::<AncestorQuery, Entity>(|entity, query| {
			query.root_ancestor(entity)
		})
		.await?;
	let out =
		BlobStore::new(FsStore::new(WsPathBuf::new("examples/bsx_site/dist")));
	let written = export_static(&world, router, &out).await?;
	Ok(format!("exported {} routes to dist", written.len()))
}
