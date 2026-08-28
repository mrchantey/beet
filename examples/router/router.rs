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
//! cargo run --example router --features http_server -- --server=http
//!
//! # REPL mode — interactive read-eval-print loop
//! cargo run --example router -- --server=repl
//! ```
//!
//! Every server is declared on the one entry root and `--server` picks which
//! ones act, so there is no argv matching in the example itself.
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		// `BeetPlugins` is a `PluginGroup`, so override its `LogPlugin` to raise the
		// level rather than re-listing the runner/router stack.
		.add_plugins(BeetPlugins.set(LogPlugin {
			level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) { commands.spawn(router_scene()); }

/// The whole entry: an entry root declaring every IO layer, with the router as
/// its dispatch child. `--server` decides which servers act, so the same scene
/// serves one command, an http listener or an interactive prompt.
pub fn router_scene() -> impl Bundle {
	(
		// the store the blob scenes read their content from
		FsStore::new(WsPathBuf::new("examples/assets")),
		servers(),
		CallOnReady::on_spawn(),
		// the batteries-included router: route lookup + the default app routes,
		// wrapping the user routes (children with a PathPartial and action)
		children![(Router::with_defaults(), children![routes()])],
	)
}

/// The IO layers, each a facet of the root's one run. Only `CliServer` boots
/// bare; the others wait to be named, so `cargo run --example router` still
/// renders once and exits.
fn servers() -> impl Bundle {
	(
		CliServer::default(),
		ReplServer {
			default_boot: false,
			..default()
		},
		#[cfg(feature = "http_server")]
		HttpServer {
			default_boot: false,
			..default()
		},
	)
}

fn routes() -> impl Bundle {
	(
		// render middleware wrapping every descendant route's content in the
		// `RouterLayout` document, transcluded in place at its `<Slot/>`
		BaseLayout::<RouterLayout>::default(),
		children![
			route::new("", BlobScene::new("content/home.md")),
			route::new("about", BlobScene::new("content/about.md")),
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
					{Increment::bundle(field_ref)}
				</div>
			}
		}),
	)
}

fn sequence() -> impl Bundle {
	route::new(
		"sequence",
		(ExchangeSequence, children![
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|cx: ActionContext<Request>| {
					info!("in sequence!");
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
