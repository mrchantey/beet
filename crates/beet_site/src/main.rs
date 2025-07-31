#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused)]
use beet::prelude::*;
use beet_site::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			BeetPlugins,
			#[cfg(feature = "launch")]
			launch_plugin,
			#[cfg(feature = "server")]
			server_plugin,
			#[cfg(feature = "client")]
			client_plugin,
		))
		.run();
}

#[cfg(feature = "launch")]
fn launch_plugin(app: &mut App) {

	let mut config = WorkspaceConfig::default();
	config.filter
		.include("*/crates/beet_design/src/**/*")
		.include("*/crates/beet_site/src/**/*");

	app.insert_resource(config);

	app.world_mut().spawn(collections());
}

#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	app.world_mut().spawn((
		children![
			pages_routes(), 
			// docs_routes(), 
			// blog_routes(), 
			// actions_routes(),
			// (
			// 	RouteFilter::new("docs"),
			// 	article_layout_middleware()
			// ),
			// (
			// 	RouteFilter::new("blog"),
			// 	article_layout_middleware()
			// ),
		],
	));
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>();
}

// #[cfg(not(feature = "client"))]
// fn main() -> Result {
// 	AppRouter::default()
// 		.add_plugins((
// 			PagesPlugin,
// 			ActionsPlugin,
// 			DocsPlugin.layer(ArticleLayout),
// 			BlogPlugin.layer(ArticleLayout),
// 			BeetDesignMockupsPlugin.layer(ArticleLayout),
// 		))
// 		.run()
// }

// #[cfg(feature = "client")]
// fn main() {
// 	App::new()
// 		.add_plugins((ApplyDirectivesPlugin, ClientIslandPlugin))
// 		.set_runner(ReactiveApp::runner)
// 		.run();
// }