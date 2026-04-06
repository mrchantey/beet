use beet::prelude::*;

pub fn routes() -> impl Bundle {
	(Name::new("Routes"), children![root(), about(), counter()])
}

fn root() -> impl Bundle {
	file_scene_tool("", "examples/router/content/home.md")
}

fn about() -> impl Bundle {
	file_scene_tool("about", "examples/router/content/about.md")
}

#[derive(Reflect)]
struct CounterParams {
	/// the number to start with
	starting_value: u32,
}


fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);
	(
		ParamsPartial::new::<CounterParams>(),
		scene_route("counter", move || {
			let field_ref = field_ref.clone();
			(Element::new("div"), children![
				(Element::new("h1"), children![Value::Str(
					"Cookie Counter".into()
				)]),
				(Element::new("p"), children![
					Value::Str("Cookie Counter: ".into()),
					field_ref.clone().as_text(),
				]),
				increment(field_ref),
			])
		}),
	)
}
