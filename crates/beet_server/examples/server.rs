use beet_core::prelude::*;
use beet_rsx::as_beet::*;
use beet_server::prelude::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.add_plugins(AppRouterPlugin)
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands.spawn((children![
		(
			RouteInfo::get("/"),
			RouteHandler::new_bundle(|| {
				rsx! {<div>hello world!</div>}
			})
		),
		(
			RouteInfo::get("/hello"),
			RouteLayer::before_route(|mut req: ResMut<Request>| {
				req.set_body("jimmy");
			}),
			RouteHandler::new(|req: Res<Request>| {
				let body = req.body_str().unwrap_or_default();
				format!("hello {}", body)
			})
		),
	],));
}
