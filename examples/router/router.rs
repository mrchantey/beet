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
		FsStore::new(WsPathBuf::new("examples/assets")),
		router_scene()?,
	));
	Ok(())
}



#[allow(unused, reason = "module not used when deploying infra")]
pub fn router_scene() -> Result<impl Bundle> {
	(
		// declare the store used by the blob scenes
		// the server is the IO layer, handling incoming requests
		// from http, stdin etc
		server_from_cli()?,
		// the batteries-included router: route lookup + the default app routes,
		// wrapping the user routes (children with a PathPartial and action)
		(default_router(), children![routes()]),
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
		"http" => HttpServer::default().any_bundle(),
		#[cfg(not(feature = "http_server"))]
		"http" => bevybail!("Add the 'http_server' feature for http servers"),
		"repl" => ReplServer::default().any_bundle(),
		"cli" => CliServer::default().any_bundle(),
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
		// scene middleware can intercept a scene route before render,
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
	let field_ref = FieldRef::new("count").with_init(0);
	(
		ParamsPartial::new::<CounterParams>(),
		render_action::fixed_route("counter", rsx_direct!{
			<div>
				<h1>"Cookie Counter"</h1>
				<p>"Value: "{field_ref.clone()}</p>
				{increment(field_ref)}
			</div>
		}),
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

/// Scene middleware that wraps a content render root in a layout template.
///
/// Calls the inner handler via [`Next`] to obtain the content render root,
/// then parses `default-layout.html` into an ephemeral entity tree and wires
/// up named [`SlotContainer`] for head, nav, and main content. The layout
/// entity becomes the coordination [`RenderRoot`], cleaning up the layout,
/// head, nav and article-header entities plus the content's own ephemerals.
/// Non-scene middleware (ie `Request/Response`) is unaffected.
///
/// Loads assets from the nearest ancestor [`BlobStore`] on each request,
/// supporting both local filesystem and S3 backends.
#[action]
#[derive(Default, Clone, Component)]
async fn LayoutTemplate(
	cx: ActionContext<(RequestParts, Next<RequestParts, Entity>)>,
) -> Result<Entity> {
	let caller = cx.caller.clone();
	let (parts, next) = cx.take();
	let path: SmolPath = parts.path().into();

	// get the content render root from the inner handler
	let content_root = next.call(parts).await?;

	// read layout and slot content from the assets BlobStore
	let store = caller
		.with_state::<AncestorQuery<&BlobStore>, BlobStore>(|entity, query| {
			query
				.get(entity)
				.cloned()
				.unwrap_or_else(|_| BlobStore::new(FsStore::default()))
		})
		.await?;
	let layout_bytes =
		store.get(&"layouts/default-layout.html".into()).await?;
	let layout_html = String::from_utf8(layout_bytes.to_vec())?;
	let head_html = head_content(&store).await?;
	let nav_html = nav_content();

	let caller_entity = caller.id();
	let world = caller.world();

	// parse layout, head, and nav into entities, wire up slots, then make the
	// layout the coordination render root
	let layout_id = world
		.with(move |world: &mut World| -> Result<Entity> {
			// the content render root names what to slot in and what to clean up
			let (content_id, content_despawn) = {
				let entity = world.entity(content_root);
				let rendered = entity
					.get::<RenderRoot>()
					.ok_or_else(|| {
						bevyhow!("inner handler did not yield a render root")
					})?
					.rendered();
				let despawn = entity
					.get::<DespawnAfterRender>()
					.map(|despawn| despawn.0.clone())
					.unwrap_or_default();
				(rendered, despawn)
			};

			let layout_id = parse_html_entity(world, &layout_html)?;
			let head_id = parse_html_entity(world, &head_html)?;
			let nav_id = parse_html_entity(world, &nav_html)?;

			// find named <slot> elements and wire up SlotContainer
			if let Some(slot) = find_named_slot(world, layout_id, "head") {
				world.entity_mut(slot).insert(SlotContainer::new(head_id));
			}
			if let Some(slot) = find_named_slot(world, layout_id, "nav") {
				world.entity_mut(slot).insert(SlotContainer::new(nav_id));
			}
			if let Some(slot) = find_named_slot(world, layout_id, "sidebar") {
				let sidebar_state = SidebarState::new(path);
				let nodes =
					world.with_state::<RouteQuery, Result<_>>(move |query| {
						let tree = query.route_tree(caller_entity)?;
						sidebar_state.collect(&tree).xok()
					})?;
				let sidebar_id =
					world.spawn_scene(rsx! { <Sidebar nodes=nodes/> })?.id();
				world.entity_mut(slot).insert(SlotContainer::new(sidebar_id));
			}
			// build article header from frontmatter if present
			let article_header_id = world
				.entity(content_id)
				.get::<Frontmatter>()
				.map(|fm| article_header_html(fm))
				.filter(|html| !html.is_empty())
				.map(|html| parse_html_entity(world, &html))
				.transpose()?;

			if let Some(slot) =
				find_named_slot(world, layout_id, "article-header")
			{
				if let Some(header_id) = article_header_id {
					world.entity_mut(slot).insert(SlotContainer::new(header_id));
				}
			}
			if let Some(slot) = find_named_slot(world, layout_id, "main") {
				world.entity_mut(slot).insert(SlotContainer::new(content_id));
			}

			// assemble the ephemeral cleanup list, extending the content's own
			let mut to_despawn = vec![layout_id, head_id, nav_id];
			if let Some(header_id) = article_header_id {
				to_despawn.push(header_id);
			}
			to_despawn.extend(content_despawn);

			let mut layout = world.entity_mut(layout_id);
			RenderRoot::insert(&mut layout, to_despawn);
			Ok(layout_id)
		})
		.await?;

	layout_id.xok()
}

/// Parses an HTML string into a new entity.
fn parse_html_entity(world: &mut World, html: &str) -> Result<Entity> {
	let entity = world.spawn_empty().id();
	let bytes = MediaBytes::new_html(html);
	let mut entity_mut = world.entity_mut(entity);
	MediaParser::new().parse(ParseContext::new(&mut entity_mut, &bytes))?;
	Ok(entity)
}

/// Generates `<head>` content including the theme switcher script.
async fn head_content(store: &BlobStore) -> Result<String> {
	let theme_bytes =
		store.get(&"js/minimal-theme-switcher.js".into()).await?;
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
