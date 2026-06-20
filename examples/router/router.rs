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
	commands
		.spawn((
			FsStore::new(WsPathBuf::new("examples/assets")),
			router_scene()?,
		))
		.trigger(ActionIn::boot);
	Ok(())
}

#[allow(unused, reason = "module not used when deploying infra")]
pub fn router_scene() -> Result<impl Bundle> {
	(
		// declare the store used by the blob scenes
		// the server is the IO layer, handling incoming requests
		// from http, stdin etc
		// the spawn site boots (empty filter matches it).
		// `ReplServer` self-boots its own loop, so it ignores the boot.
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
		.map(|val: &SmolStr| val.to_lowercase())
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
		// render middleware wrapping every descendant route's content in the
		// `RouterLayout` document, transcluded in place at its `<Slot/>`
		BaseLayout::<RouterLayout>::default(),
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
	(
		ParamsPartial::new::<CounterParams>(),
		render_action::fixed_func_route("counter", || {
			let field_ref = FieldRef::new("count").with_init(0);
			rsx! {
				<div>
					<h1>"Cookie Counter"</h1>
					<p>"Value: "{field_ref.clone()}</p>
					{increment(field_ref)}
				</div>
			}
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
// ║   Layout                                  ║
// ╚═══════════════════════════════════════════╝

/// The document layout wrapping every route's content.
///
/// An ordinary `#[template]` widget with a `<Slot/>`: the [`BaseLayout`] render
/// middleware runs each route, then transcludes the resulting content in place
/// at the `<Slot/>`. The `<head>` is non-visual, so the same layout renders in
/// the terminal and over HTTP.
#[template]
fn RouterLayout() -> impl Bundle {
	rsx! {
		<html>
			<head><title>"Router Example"</title></head>
			<body>
				<nav>
					<ul>
						<li><a href="/">"Home"</a></li>
						<li><a href="/about">"About"</a></li>
						<li><a href="/counter">"Counter"</a></li>
					</ul>
				</nav>
				<main><Slot/></main>
			</body>
		</html>
	}
}
