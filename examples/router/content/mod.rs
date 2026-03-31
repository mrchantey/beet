use beet::prelude::*;

/// Pete's Beets — a music record store stack driven by markdown files.
///
/// Each scene loads its content from a `.md` file via [`file_scene_tool`],
/// which reads and parses markdown on each request.
pub fn stack() -> impl Bundle {
	(default_router(), children![root(), about(), counter()])
}

fn root() -> impl Bundle {
	file_scene_tool("", "examples/router/content/home.md")
}

fn about() -> impl Bundle {
	file_scene_tool("about", "examples/router/content/about.md")
}

/// Stock counter page using document fields and tools.
fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);
	scene_route("counter", move || {
		let field_ref = field_ref.clone();
		(Element::new("div"), children![
			(Element::new("h1"), children![Value::Str(
				"Stock Counter".into()
			)]),
			(Element::new("p"), children![
				Value::Str("Cookie Counter: ".into()),
				field_ref.clone().as_text(),
			]),
			increment(field_ref),
		])
	})
}
