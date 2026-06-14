//! Showcase gallery — a runnable page exercising the Material Design 3 rule
//! set and the ported `beet_ui` widgets. The whole page (document layout,
//! stylesheet, and gallery body) is a single template: no hand-assembled HTML
//! string. The built stylesheet is itself a widget ([`Stylesheet`]), the color
//! scheme seeds via [`ColorSchemeScript`], and the body is rendered once and
//! served over HTTP for live inspection (and also written to disk).
//!
//! Run with:
//! ```not_rust
//! cargo run -p beet_ui --example showcase --features template,style
//! ```
//! then open <http://localhost:8337>.
//!
//! `<Select>`/`<Table>` options and rows are passed as slotted children: caller
//! content written between the tags lands in the widget's default `<Slot>`, and
//! `slot="head"` rows route to the named `head` slot (see `<Slot>`).
use beet_core::prelude::*;
use beet_net::prelude::DEFAULT_SERVER_PORT;
use beet_net::prelude::HttpServer;
use beet_net::prelude::MediaType;
use beet_net::prelude::Response;
use beet_net::prelude::ServerPlugin;
use beet_net::prelude::exchange_handler;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use bevy::MinimalPlugins;
use bevy::app::App;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			TemplatePlugin,
			DocumentPlugin,
			material::MaterialStylePlugin::new(palettes::basic::BLUE),
			ServerPlugin,
		))
		.insert_resource(PackageConfig {
			title: "Beet UI Showcase".into(),
			description: "A gallery of beet_ui rules and widgets".into(),
			binary_name: Some("showcase".into()),
			version: "0.0.0".into(),
			homepage: Some("https://beetstack.dev".into()),
			repository: None,
			stage: "dev".into(),
			service_access: ServiceAccess::Local,
		})
		.add_systems(Startup, serve_showcase)
		.run();
}

/// Render the showcase page once, write it to disk, and serve it on every route.
fn serve_showcase(world: &mut World) -> Result {
	let root = world.spawn_template(Snippet::from_bundle(showcase_page()))?.id();
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
		exchange_handler(move |_| {
			Response::ok_body(html.clone(), MediaType::Html)
		}),
	));

	info!(
		"Showcase served at http://localhost:{port} (also written to {})",
		path.display()
	);
	Ok(())
}

/// The full page as one scene: an `<html>` layout whose `<head>` carries the
/// preflight reset, the built [`Stylesheet`], and the [`ColorSchemeScript`],
/// with the [`gallery`] in the `<body>`.
fn showcase_page() -> impl Bundle {
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
fn gallery() -> impl Bundle {
	rsx! {
		<main {Classes::new([classes::PAGE])}>
			<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>"Beet UI Showcase"</h1>

			// ── Buttons (widgets) ─────────────────────────────────────────────
			<section>
				<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Buttons"</h2>
				<Button variant=ButtonVariant::Filled>"Filled"</Button>
				<Button variant=ButtonVariant::Outlined>"Outlined"</Button>
				<Button variant=ButtonVariant::Text>"Text"</Button>
				<Button variant=ButtonVariant::Tonal>"Tonal"</Button>
				<Button variant=ButtonVariant::Elevated>"Elevated"</Button>
				<Button variant=ButtonVariant::Secondary>"Secondary"</Button>
				<Button variant=ButtonVariant::Tertiary>"Tertiary"</Button>
				<Button variant=ButtonVariant::Error>"Error"</Button>
				<IconButton>"+"</IconButton>
				<Link href="#" variant=ButtonVariant::Text>"A link button"</Link>
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
				// options become the Select's `children` prop
				<Select name="fruit">
					<option>"Apple"</option>
					<option>"Banana"</option>
				</Select>
				<ErrorText message="This field is required"/>
			</section>

			// ── Table (rows route to the head / default props) ────────────────
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
