use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;

fn main() {
	App::new()
		.add_plugins((material::MaterialStylePlugin::new(
			palettes::basic::YELLOW,
		),))
		.add_systems(Startup, setup)
		.run();
}

fn setup(
	ruleset: Res<RuleSet>,
	query: StyleQuery,
	mut commands: Commands,
) -> Result {
	let entity = commands.spawn_empty().id();

	let ruleset_path =
		AbsPathBuf::new_workspace_rel("target/examples/style/ruleset.json")
			.unwrap();

	// write the store for inspection
	let json = serde_json::to_string_pretty(&*ruleset).unwrap();
	fs_ext::write(&ruleset_path, json).unwrap();
	println!("Ruleset written to: {}", ruleset_path.display());

	let css = query.build_css(
		&CssBuilder::default()
			// .with_format_variables(FormatVariables::Full),
			// .with_format_variables(FormatVariables::Hash { min_len: 1 }),
			.with_format_variables(FormatVariables::short()),
		entity,
	)?;

	let html = format!(
		r#"<!DOCTYPE html>
<html>
<head>
	<meta charset="utf-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>Beet Style</title>
	<link
		rel="stylesheet"
		href="https://unpkg.com/tailwindcss@4/preflight.css"
	/>
	<style>{css}</style>
</head>
<body>
	<header>Hello World!</header>
</body>
</html>"#
	);

	let html_path =
		AbsPathBuf::new_workspace_rel("target/examples/style/index.html")
			.unwrap();

	// write the html for inspection
	fs_ext::write(&html_path, &html)?;

	println!("HTML written to: {}", html_path.display());
	Ok(())
}
