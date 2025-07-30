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
	app.world_mut().spawn((
		RouteCodegenRoot::default(),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("crates/beet_site/src/codegen/mod.rs")
				.unwrap(),
		),
		children![
			pages_collection(), 
			docs_collection(), 
			actions_collection()
		],
	));
}

#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	app.world_mut().spawn((
		children![
			pages_routes(), 
			docs_routes(), 
			actions_routes()
		],
		// this is placed last to ensure it runs after all handlers
		RouteHandler::layer(|| {
			let mut state = AppState::get();
			state.num_requests += 1;
			AppState::set(state);
		}),
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