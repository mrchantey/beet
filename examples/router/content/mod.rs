use beet::prelude::*;

pub fn routes() -> impl Bundle {
	// Nest under a child entity so the MiddlewareList only applies
	// to route descendants, not the server entity's own exchange fallback.
	(Middleware::<LayoutTemplate, _, _>::default(), children![
		route("", FileScene::new("examples/router/content/home.md")),
		route("about", FileScene::new("examples/router/content/about.md")),
		counter()
	])
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
		fixed_scene(
			"counter",
			(Element::new("div"), children![
				Element::new("h1").with_inner_text("Cookie Counter"),
				(Element::new("p"), children![
					Value::Str("Cookie Counter: ".into()),
					field_ref.clone().as_text(),
				]),
				increment(field_ref),
			]),
		),
	)
}

// ╔═══════════════════════════════════════════╗
// ║   Layout Template Middleware              ║
// ╚═══════════════════════════════════════════╝

/// Scene middleware that wraps a [`SceneEntity`] in a layout template.
///
/// Calls the inner handler via [`Next`] to obtain the content scene,
/// then parses `default-layout.html` into an ephemeral entity tree
/// and wires up named [`SlotContainer`] for head, nav, and main content.
/// Non-scene middleware (ie `Request/Response`) is unaffected.
///
/// Loads assets from disk on each request so templates can be edited
/// without restarting the server.
#[action]
#[derive(Default, Clone, Component)]
async fn LayoutTemplate(
	cx: ActionContext<(RequestParts, Next<RequestParts, SceneEntity>)>,
) -> Result<SceneEntity> {
	let caller = cx.caller.clone();
	let (parts, next) = cx.take();
	let path: RelPath = parts.path().into();

	// get the content scene from the inner handler
	let content = next.call(parts).await?;
	let content_id = content.entity;

	// read layout and slot content from disk
	let layout_html =
		fs_ext::read_to_string("examples/assets/layouts/default-layout.html")?;
	let head_html = head_content()?;
	let nav_html = nav_content();

	let caller_entity = caller.id();
	let world = caller.world();

	// parse layout, head, and nav into entities, then wire up slots
	let (layout_id, head_id, nav_id, article_header_id) = world
		.with_then(
			move |world: &mut World| -> Result<(Entity, Entity, Entity, Option<Entity>)> {
				let layout_id = parse_html_entity(world, &layout_html, true)?;
				let head_id = parse_html_entity(world, &head_html, false)?;
				let nav_id = parse_html_entity(world, &nav_html, false)?;

				// find named <slot> elements and wire up SlotContainer
				if let Some(slot) = find_named_slot(world, layout_id, "head") {
					world.entity_mut(slot).insert(SlotContainer::new(head_id));
				}
				if let Some(slot) = find_named_slot(world, layout_id, "nav") {
					world.entity_mut(slot).insert(SlotContainer::new(nav_id));
				}
				if let Some(slot) = find_named_slot(world, layout_id, "sidebar")
				{
					let sidebar_state = SidebarState::new(path);
					let bundle = world.with_state::<RouteQuery, Result<_>>(
						move |query| {
							let tree = query.route_tree(caller_entity)?;
							let bundle = sidebar_state.build(&tree);
							bundle.xok()
						},
					)?;
					world.entity_mut(slot).insert(bundle);
				}
				// build article header from frontmatter if present
				let article_header_id = world
					.entity(content_id)
					.get::<Frontmatter>()
					.map(|fm| article_header_html(fm))
					.filter(|html| !html.is_empty())
					.map(|html| parse_html_entity(world, &html, false))
					.transpose()?;

				if let Some(slot) =
					find_named_slot(world, layout_id, "article-header")
				{
					if let Some(header_id) = article_header_id {
						world
							.entity_mut(slot)
							.insert(SlotContainer::new(header_id));
					}
				}
				if let Some(slot) = find_named_slot(world, layout_id, "main") {
					world
						.entity_mut(slot)
						.insert(SlotContainer::new(content_id));
				}

				Ok((layout_id, head_id, nav_id, article_header_id))
			},
		)
		.await?;

	// build scene entity with all ephemeral entities for cleanup
	let mut scene = SceneEntity::new_ephemeral(layout_id)
		.push_despawn(head_id)
		.push_despawn(nav_id);
	if let Some(header_id) = article_header_id {
		scene = scene.push_despawn(header_id);
	}
	scene.with_join(content).xok()
}

/// Parses an HTML string into a new entity. If `scope` is true, the
/// entity gets a [`DocumentScope`] component.
fn parse_html_entity(
	world: &mut World,
	html: &str,
	scope: bool,
) -> Result<Entity> {
	let entity = if scope {
		world.spawn(DocumentScope).id()
	} else {
		world.spawn_empty().id()
	};
	let bytes = MediaBytes::new_html(html);
	let mut entity_mut = world.entity_mut(entity);
	MediaParser::new().parse(ParseContext::new(&mut entity_mut, &bytes))?;
	Ok(entity)
}

/// Generates `<head>` content including the theme switcher script.
fn head_content() -> Result<String> {
	let theme_switcher =
		fs_ext::read_to_string("examples/assets/js/minimal-theme-switcher.js")?;
	Ok(format!(r#"<script>{theme_switcher}</script>"#))
}

/// Builds an article header HTML string from [`Frontmatter`] fields.
///
/// Renders the `title`, `created`, and `edited` fields if present.
/// Returns an empty string when no relevant fields exist.
fn article_header_html(fm: &Frontmatter) -> String {
	let title = fm.get_str("title").unwrap_or_default();
	let created = fm.get_str("created").unwrap_or_default();
	let edited = fm.get_str("edited").unwrap_or_default();

	if title.is_empty() && created.is_empty() && edited.is_empty() {
		return String::new();
	}

	let mut html = String::from("<header>");
	if !title.is_empty() {
		html.push_str(&format!("<h1>{title}</h1>"));
	}
	if !created.is_empty() || !edited.is_empty() {
		html.push_str("<p><small>");
		if !created.is_empty() {
			html.push_str(&format!("Created: {created}"));
		}
		if !created.is_empty() && !edited.is_empty() {
			html.push_str(" · ");
		}
		if !edited.is_empty() {
			html.push_str(&format!("Edited: {edited}"));
		}
		html.push_str("</small></p>");
	}
	html.push_str("</header>");
	html
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
