//! Showcase gallery — a runnable page exercising the Material Design 3 rule
//! set and the ported `beet_ui` widgets. The whole page (document shell,
//! stylesheet, and gallery body) is a single scene: no hand-assembled HTML
//! string. The built stylesheet is itself a widget ([`Stylesheet`]), the color
//! scheme seeds via [`ColorSchemeScript`], and the body is rendered once and
//! served over HTTP for live inspection (and also written to disk).
//!
//! Run with:
//! ```not_rust
//! cargo run -p beet_ui --example showcase --features scene,style
//! ```
//! then open <http://localhost:8337>.
//!
//! `<Select>`/`<Table>` options and rows flow into the widgets' slots: caller
//! content written between the tags is auto-wired into the matching `<slot>`
//! at spawn time (see `beet_ui::scene::apply_slots`).
use beet_core::prelude::*;
use beet_net::prelude::DEFAULT_SERVER_PORT;
use beet_net::prelude::HttpServer;
use beet_net::prelude::MediaType;
use beet_net::prelude::Response;
use beet_net::prelude::ServerPlugin;
use beet_net::prelude::exchange_handler;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
// explicit so `spawn_scene` resolves to beet_ui's slot-wiring trait, not the
// `bevy::scene` one also glob-imported via `beet_core::prelude`.
use beet_ui::prelude::WorldSceneExt;
use beet_ui::*;
use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::asset::AssetPlugin;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			ScenePlugin,
			material::MaterialStylePlugin::new(palettes::basic::BLUE),
			ServerPlugin,
		))
		.insert_resource(PackageConfig {
			title: "Beet UI Showcase".into(),
			binary_name: "showcase".into(),
			version: "0.0.0".into(),
			description: "A gallery of beet_ui rules and widgets".into(),
			homepage: "https://beetstack.dev".into(),
			repository: None,
			stage: "dev".into(),
			service_access: ServiceAccess::Local,
		})
		.add_systems(Startup, serve_showcase)
		.run();
}

/// Render the showcase page once, write it to disk, and serve it on every route.
fn serve_showcase(world: &mut World) -> Result {
	let root = world.spawn_scene(showcase_page())?.id();
	let html = HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))?
		.to_string();

	// write to disk for offline inspection
	let path =
		AbsPathBuf::new_workspace_rel("target/examples/showcase/index.html")?;
	fs_ext::write(&path, &html)?;

	// serve the pre-rendered page on every route
	let server = HttpServer::default();
	let port = server.port.unwrap_or(DEFAULT_SERVER_PORT);
	world.spawn((
		server,
		exchange_handler(move |_| Response::ok_body(html.clone(), MediaType::Html)),
	));

	cross_log!(
		"Showcase served at http://localhost:{port} (also written to {})",
		path.display()
	);
	Ok(())
}

/// The full page as one scene: an `<html>` shell whose `<head>` carries the
/// preflight reset, the built [`Stylesheet`], and the [`ColorSchemeScript`],
/// with the [`gallery`] in the `<body>`.
fn showcase_page() -> impl Scene {
	rsx! {
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<title>"Beet UI Showcase"</title>
				<Preflight/>
				<Stylesheet/>
				<ColorSchemeScript/>
			</head>
			<body>
				{gallery()}
			</body>
		</html>
	}
}

/// The gallery body: a `<main>` with a section per rule/widget group.
fn gallery() -> impl Scene {
	rsx! {
		<main {Classes::new([classes::PAGE])}>
			<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>"Beet UI Showcase"</h1>

			// ── Buttons (widgets) ─────────────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Buttons"</h2>
				<Button label="Filled" variant=ButtonVariant::Filled/>
				<Button label="Outlined" variant=ButtonVariant::Outlined/>
				<Button label="Text" variant=ButtonVariant::Text/>
				<Button label="Tonal" variant=ButtonVariant::Tonal/>
				<Button label="Elevated" variant=ButtonVariant::Elevated/>
				<Button label="Secondary" variant=ButtonVariant::Secondary/>
				<Button label="Tertiary" variant=ButtonVariant::Tertiary/>
				<Button label="Error" variant=ButtonVariant::Error/>
				<IconButton label="+"/>
				<Link label="A link button" href="#" variant=ButtonVariant::Text/>
			</section>

			// ── Cards ─────────────────────────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Cards"</h2>
				<div {Classes::new([classes::CARD_FILLED])}>"Filled card"</div>
				<div {Classes::new([classes::CARD_ELEVATED])}>"Elevated card"</div>
				<div {Classes::new([classes::CARD_OUTLINED])}>"Outlined card"</div>
			</section>

			// ── Typography scale ──────────────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Typography"</h2>
				<p {Classes::new([classes::TEXT_DISPLAY_LARGE])}>"Display large"</p>
				<p {Classes::new([classes::TEXT_HEADLINE_LARGE])}>"Headline large"</p>
				<p {Classes::new([classes::TEXT_TITLE_LARGE])}>"Title large"</p>
				<p {Classes::new([classes::TEXT_BODY_LARGE])}>"Body large"</p>
				<p {Classes::new([classes::TEXT_LABEL_LARGE])}>"Label large"</p>
			</section>

			// ── Form controls (widgets) ───────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Form controls"</h2>
				<TextField name="email" placeholder="Email" variant=TextFieldVariant::Outlined/>
				<TextField name="filled" placeholder="Filled" variant=TextFieldVariant::Filled/>
				<TextArea name="bio" placeholder="Bio"/>
				// options flow into the Select's default slot
				<Select name="fruit">
					<option>"Apple"</option>
					<option>"Banana"</option>
				</Select>
				<ErrorText message="This field is required"/>
			</section>

			// ── Table (rows flow into head/default slots) ─────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Table"</h2>
				<Table>
					<tr slot="head"><th>"Name"</th><th>"Role"</th></tr>
					<tr><td>"Ada"</td><td>"Engineer"</td></tr>
					<tr><td>"Grace"</td><td>"Admiral"</td></tr>
				</Table>
			</section>

			// ── Sidebar (widget) ──────────────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Sidebar"</h2>
				<Sidebar nodes=sidebar_nodes()/>
			</section>
		</main>
	}
}

/// A small nav tree for the [`Sidebar`] widget.
fn sidebar_nodes() -> Vec<SidebarNode> {
	vec![
		SidebarNode {
			display_name: "Home".into(),
			path: Some(SmolPath::new("/")),
			..default()
		},
		SidebarNode {
			display_name: "Docs".into(),
			path: None,
			expanded: true,
			children: vec![
				SidebarNode {
					display_name: "Intro".into(),
					path: Some(SmolPath::new("docs/intro")),
					..default()
				},
				SidebarNode {
					display_name: "API".into(),
					path: Some(SmolPath::new("docs/api")),
					..default()
				},
			],
			..default()
		},
	]
}
