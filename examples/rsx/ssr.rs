//! An example of basic server-side rendering (SSR) with beet.
//!
//! ```sh
//! cargo run --example ssr --features=server,css
//! ```
use beet::prelude::*;
use std::sync::LazyLock;
use std::sync::Mutex;

fn main() {
	App::new()
		.add_plugins(BeetPlugins)
		.add_systems(Startup, setup)
		.insert_resource(RenderMode::Ssr)
		.run();
}

// #[rustfmt::skip]
fn setup(mut commands: Commands) {
	commands.insert_resource(Router::new_no_defaults(|app: &mut App| {
		app.init_plugin::<HandlerPlugin>();
		app.world_mut().spawn((RouterRoot, children![
			// bundles are served as html documents
			(PathFilter::new("/"), bundle_endpoint(|| rsx! {<Home/>})),
			// common types implement IntoResponse
			(PathFilter::new("/foo"), RouteHandler::endpoint(|| "bar")),
			// middleware example
			(
				PathFilter::new("/hello-layer"),
				// children are run in sequence
				children![
					RouteHandler::layer(modify_request),
					bundle_endpoint(|req: In<Request>| {
						// let body = req.body_str().unwrap_or_default();
						todo!();
						let body = String::new();
						rsx! {
							<Style/>
							<main>
								<div> hello {body}</div>
								<a href="/">go home</a>
							</main>
						}
					}),
					RouteHandler::layer(modify_response),
				]
			)
		]));
	}));
}

// modifies the request body to "jimmy"
fn modify_request(mut req: ResMut<Request>) { req.set_body("jimmy"); }
fn modify_response(world: &mut World) {
	let entity = world.query_filtered_once::<Entity, With<HandlerBundle>>()[0];

	world.spawn((HtmlDocument, rsx! {
		<Style/>
		<article>
		<h1>Warm greetings!</h1>
			{entity}
		</article>
	}));
}


#[template]
fn Home() -> impl Bundle {
	let mut state = AppState::get();
	let uptime = state.started.elapsed();
	let uptime = format!("{:.2}", uptime.as_secs_f32());
	let num_requests = state.num_requests;

	state.num_requests += 1;
	AppState::set(state);
	rsx! {
		<Style/>
		<main>
			<div>hello world!</div>
			<div>uptime: {uptime} seconds</div>
			<div>request count: {num_requests}</div>
			<a href="/hello-layer">visit jimmy</a>
			{
				match num_requests % 7 {
					0 => rsx! {
						<div>
						Congratulations you are visitor number {num_requests}!
						</div>
					}.any_bundle(),
					_ => ().any_bundle(),
				}
			}
		</main>
	}
}

#[derive(Clone)]
struct AppState {
	started: std::time::Instant,
	num_requests: u32,
}
impl AppState {
	pub fn get() -> AppState { APP_STATE.lock().unwrap().clone() }
	pub fn set(state: AppState) { *APP_STATE.lock().unwrap() = state; }
}
static APP_STATE: LazyLock<Mutex<AppState>> = LazyLock::new(|| {
	Mutex::new(AppState {
		started: std::time::Instant::now(),
		num_requests: 0,
	})
});




#[template]
fn Style() -> impl Bundle {
	// css is much easier to write with the rsx_combinator macro
	// as many common css tokens like `1em` or `a:visited` are not valid rust tokens
	rsx_combinator! {r"
<style scope:global>
	main,article {
		padding-top: 2em;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 100vh;
	}
	a {
		color: #90ee90;
	}
	a:visited {
		color: #3399ff;
	}
	body {
		font-size: 1.4em;
		font-family: system-ui, sans-serif;
		background: black;
		color: white;
	}
</style>"}
}
