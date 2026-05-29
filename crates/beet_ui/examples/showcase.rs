//! Showcase gallery — a runnable page exercising the Material Design 3 rule
//! set and the ported `beet_ui` widgets. Builds the full stylesheet from the
//! active [`RuleSet`] and a gallery body (swatches, typography, buttons, cards,
//! form controls, a table and a sidebar), then writes a self-contained
//! `index.html` for visual inspection.
//!
//! Run with:
//! ```not_rust
//! cargo run -p beet_ui --example showcase --features scene,style
//! ```
//!
//! This is the first real visual-verification loop for the
//! `beet_design` → `beet_ui` migration (see `agent/plans/beet_design.md`).
use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use beet_ui::*;
use bevy::app::TaskPoolPlugin;
use bevy::asset::AssetPlugin;
use bevy::scene::ScenePlugin;

fn main() {
	// a world that can both build CSS (MaterialStylePlugin) and spawn scenes
	// (ScenePlugin + AssetServer)
	let mut world = (
		TaskPoolPlugin::default(),
		AssetPlugin::default(),
		ScenePlugin,
		material::MaterialStylePlugin::new(palettes::basic::BLUE),
	)
		.into_world();
	world.insert_resource(PackageConfig {
		title: "Beet UI Showcase".into(),
		binary_name: "showcase".into(),
		version: "0.0.0".into(),
		description: "A gallery of beet_ui rules and widgets".into(),
		homepage: "https://beetstack.dev".into(),
		repository: None,
		stage: "dev".into(),
		service_access: ServiceAccess::Local,
	});

	let body = world.spawn_scene(gallery()).unwrap().id();
	let body_html = HtmlRenderer::new()
		.render(&mut RenderContext::new(body, &mut world))
		.unwrap()
		.to_string();

	let css = world
		.with_state::<StyleQuery, _>(|query| {
			query.build_css(
				&CssBuilder::default()
					.with_minify(false)
					.with_format_variables(FormatVariables::short()),
			)
		})
		.unwrap();

	let html = format!(
		r#"<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="utf-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>Beet UI Showcase</title>
	<link rel="stylesheet" href="https://unpkg.com/tailwindcss@4/preflight.css"/>
	<style>{css}</style>
</head>
<body class="light-scheme">
{body_html}
</body>
</html>"#
	);

	let path =
		AbsPathBuf::new_workspace_rel("target/examples/showcase/index.html")
			.unwrap();
	fs_ext::write(&path, &html).unwrap();
	println!("Showcase written to: {}", path.display());
}

/// The full gallery scene: a `<main>` with a section per rule/widget group.
fn gallery() -> impl Scene {
	rsx! {
		<main class="page">
			<h1 class="text-display-medium">"Beet UI Showcase"</h1>

			// ── Buttons (widgets) ─────────────────────────────────────────────
			<section>
				<h2 class="text-headline-small">"Buttons"</h2>
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
				<h2 class="text-headline-small">"Cards"</h2>
				<div class="card-filled">"Filled card"</div>
				<div class="card-elevated">"Elevated card"</div>
				<div class="card-outlined">"Outlined card"</div>
			</section>

			// ── Typography scale ──────────────────────────────────────────────
			<section>
				<h2 class="text-headline-small">"Typography"</h2>
				<p class="text-display-large">"Display large"</p>
				<p class="text-headline-large">"Headline large"</p>
				<p class="text-title-large">"Title large"</p>
				<p class="text-body-large">"Body large"</p>
				<p class="text-label-large">"Label large"</p>
			</section>

			// ── Form controls (widgets) ───────────────────────────────────────
			<section>
				<h2 class="text-headline-small">"Form controls"</h2>
				<TextField name="email" placeholder="Email" variant=TextFieldVariant::Outlined/>
				<TextField name="filled" placeholder="Filled" variant=TextFieldVariant::Filled/>
				<TextArea name="bio" placeholder="Bio"/>
				// Select options use raw markup (slot wiring is still pending)
				<select class="select select-outlined" name="fruit">
					<option>"Apple"</option>
					<option>"Banana"</option>
				</select>
				<ErrorText message="This field is required"/>
			</section>

			// ── Table (raw markup; slot wiring pending) ───────────────────────
			<section>
				<h2 class="text-headline-small">"Table"</h2>
				<table class="table">
					<thead>
						<tr><th>"Name"</th><th>"Role"</th></tr>
					</thead>
					<tbody>
						<tr><td>"Ada"</td><td>"Engineer"</td></tr>
						<tr><td>"Grace"</td><td>"Admiral"</td></tr>
					</tbody>
				</table>
			</section>

			// ── Sidebar (widget) ──────────────────────────────────────────────
			<section>
				<h2 class="text-headline-small">"Sidebar"</h2>
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
			path: Some(RelPath::new("/")),
			..default()
		},
		SidebarNode {
			display_name: "Docs".into(),
			path: None,
			expanded: true,
			children: vec![
				SidebarNode {
					display_name: "Intro".into(),
					path: Some(RelPath::new("docs/intro")),
					..default()
				},
				SidebarNode {
					display_name: "API".into(),
					path: Some(RelPath::new("docs/api")),
					..default()
				},
			],
			..default()
		},
	]
}
