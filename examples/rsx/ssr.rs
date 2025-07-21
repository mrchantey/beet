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
		.add_plugins(AppRouterPlugin)
		.add_systems(Startup, setup)
		.run();
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



fn setup(mut commands: Commands) {
	commands.spawn((children![
		(
			StaticRoute,
			RouteInfo::get("/"),
			RouteHandler::new_bundle(|| rsx! {<Home/>})
		),
		(
			StaticRoute,
			RouteInfo::get("/hello-layer"),
			// layers are regular systems that run before or after the route handler
			RouteLayer::before_route(|mut req: ResMut<Request>| {
				req.set_body("jimmy");
			}),
			RouteHandler::new_bundle(|req: Res<Request>| {
				let body = req.body_str().unwrap_or_default();
				rsx! {
					<Style/>
					<main>
						<div> hello {body}</div>
						<a href="/">go home</a>
					</main>
				}
			})
		),
	],));
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


#[template]
fn Style() -> impl Bundle {
	// css is much easier to write with the rsx_combinator!,
	// as many css tokens are not valid rust tokens, ie 1em, a:visited, etc.
	rsx_combinator! {r"
<style scope:global>
	main{
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
</style>
<style scope:global>
	body{
		font-size: 1.4em;
		font-family: system-ui, sans-serif;
		background: black;
		color: white;
	}
</style>"}
}
