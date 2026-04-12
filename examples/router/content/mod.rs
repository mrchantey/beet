use beet::prelude::*;

pub fn routes() -> impl Bundle {
	// Nest under a child entity so the MiddlewareList only applies
	// to route descendants, not the server entity's own exchange fallback.
	children![(
		Name::new("Routes"),
		Middleware::<LayoutTemplate, _, _>::default(),
		children![
			route("", FileScene::new("examples/router/content/home.md")),
			route("about", FileScene::new("examples/router/content/about.md")),
			counter()
		]
	)]
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
		scene_func("counter", move || {
			let field_ref = field_ref.clone();
			(Element::new("div"), children![
				Element::new("h1").with_inner_text("Cookie Counter"),
				(Element::new("p"), children![
					Value::Str("Cookie Counter: ".into()),
					field_ref.clone().as_text(),
				]),
				increment(field_ref),
			])
		}),
	)
}

// ╔═══════════════════════════════════════════╗
// ║   Layout Template Middleware              ║
// ╚═══════════════════════════════════════════╝

/// Middleware that wraps HTML responses in a layout template.
///
/// Calls the inner handler via [`Next`], then checks the response content type.
/// HTML responses are wrapped in `default-layout.html` with injected head,
/// navigation and main content. Non-HTML responses pass through unchanged.
///
/// Loads assets from disk on each request so templates can be edited without
/// restarting the server.
#[tool]
#[derive(Default, Clone, Component)]
async fn LayoutTemplate(
	cx: ToolContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let (request, next) = cx.take();
	let response = next.call(request).await?;

	let content_type = response
		.parts
		.headers
		.get::<header::ContentType>()
		.and_then(|result| result.ok());

	if content_type != Some(MediaType::Html) {
		return Ok(response);
	}

	let (parts, body) = response.into_parts();
	let body_text = body.into_string().await?;
	let wrapped = render_layout(&body_text);
	Ok(Response {
		parts,
		body: wrapped.into(),
	})
}

/// Wraps content in the default layout, injecting head, nav, and main slots.
fn render_layout(main_content: &str) -> String {
	fs_ext::read_to_string("examples/assets/layouts/default-layout.html")
		.unwrap()
		.replace("{{ head }}", &head_content())
		.replace("{{ nav }}", &nav_content())
		.replace("{{ main }}", main_content)
}

/// Generates `<head>` content including the theme switcher script.
fn head_content() -> String {
	let theme_switcher =
		fs_ext::read_to_string("examples/assets/js/minimal-theme-switcher.js")
			.unwrap();
	format!(r#"<script>{theme_switcher}</script>"#)
}

/// Generates navigation `<li>` items from the known routes.
fn nav_content() -> String {
	[("Home", "/"), ("About", "/about"), ("Counter", "/counter")]
		.iter()
		.map(|(label, path)| {
			format!(r#"<li><a href="{}">{}</a></li>"#, path, label)
		})
		.collect::<String>()
}
