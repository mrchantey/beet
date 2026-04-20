//! # Router Example
//!
//! Demonstrates beet's routing system with multiple server backends.
//!
//! ## Running the Example
//!
//! ```sh
//! # CLI mode (default) — show root content and exit
//! cargo run --example router
//!
//! # CLI mode — show help for all routes
//! cargo run --example router -- --help
//!
//! # CLI mode — navigate to a scene
//! cargo run --example router -- about
//!
//! # CLI mode — show help scoped to a subcommand
//! cargo run --example router -- counter --help
//!
//! # CLI mode — request HTML output wrapped in the layout template
//! cargo run --example router -- --accept=text/html
//! cargo run --example router -- about --accept=text/html
//!
//! # HTTP mode — start an HTTP server on port 8337
//! cargo run --example router --features http_server
//!
//! # REPL mode — interactive read-eval-print loop
//! cargo run --example router -- --server=repl
//! ```
use beet::prelude::*;

#[allow(unused, reason = "module used by hello_lambda etc")]
fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				level: Level::TRACE,
				..default()
			},
			ClientAppPlugin,
		))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) -> Result {
	commands.spawn((
		FsBucket::new(WsPathBuf::new("examples/assets")),
		router_scene()?,
	));
	Ok(())
}



#[allow(unused)]
pub fn router_scene() -> Result<impl Bundle> {
	(
		// declare the bucket used by the blob scenes
		// the server is the IO layer, handling incoming requests
		// from http, stdin etc
		server_from_cli()?,
		// the router will handle the request, shortcircuiting
		// on a `help` param, otherwise directing to a child route
		router(),
		// the actual routes, children with a PathPartial and associated action
		// for handling a request
		routes(),
	)
		.xok()
}


// OnSpawn serves as a type erased bundle
fn server_from_cli() -> Result<OnSpawn> {
	cfg_if! {
		if #[cfg(feature="http_server")]{
			let default_server = "http";
		}else{
			let default_server = "cli";
		}
	};


	match CliArgs::parse_env()
		.params
		.get("server")
		.map(|val: &String| val.to_lowercase())
		.unwrap_or_else(|| default_server.into())
		.as_str()
	{
		// use on_spawn to avoid clobbering children!
		#[cfg(feature = "http_server")]
		"http" => OnSpawn::insert(HttpServer::default()),
		#[cfg(not(feature = "http_server"))]
		"http" => bevybail!("Add the 'http_server' feature for http servers"),
		"repl" => OnSpawn::insert(ReplServer::default()),
		"cli" => OnSpawn::insert(CliServer::default()),
		_ => {
			bevybail!(
				"Invalid server type specified. Accepted options are http,repl,cli"
			);
		}
	}
	.xok()
}


fn routes() -> impl Bundle {
	(
		// SceneEntity middleware can intercept a scene route before render,
		// useful for applying a layout
		Middleware::<LayoutTemplate, _, _>::default(),
		children![
			route("", BlobScene::new("content/home.md")),
			route("about", BlobScene::new("content/about.md")),
			counter(),
			sequence()
		],
	)
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



fn sequence() -> impl Bundle {
	route(
		"sequence",
		(exchange_sequence(), children![
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|cx: ActionContext<Request>| {
					println!("in sequence!");
					Pass(cx.input)
				},
			),
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|_cx: ActionContext<Request>| {
					Fail(Response::ok().with_body("Sequence complete!"))
				}
			)
		]),
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
/// Loads assets from the nearest ancestor [`Bucket`] on each request,
/// supporting both local filesystem and S3 backends.
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

	// read layout and slot content from the assets Bucket
	let bucket = caller
		.with_state::<AncestorQuery<&Bucket>, Bucket>(|entity, query| {
			query
				.get(entity)
				.cloned()
				.unwrap_or_else(|_| Bucket::new(FsBucket::default()))
		})
		.await;
	let layout_bytes =
		bucket.get(&"layouts/default-layout.html".into()).await?;
	let layout_html = String::from_utf8(layout_bytes.to_vec())?;
	let head_html = head_content(&bucket).await?;
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
async fn head_content(bucket: &Bucket) -> Result<String> {
	let theme_bytes =
		bucket.get(&"js/minimal-theme-switcher.js".into()).await?;
	let theme_switcher = String::from_utf8(theme_bytes.to_vec())?;
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
