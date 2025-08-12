use crate::prelude::*;


pub fn server_plugin(app: &mut App) {
	// create a router, specifying the plugin for Router Apps
	app.insert_resource(Router::new_bundle(routes));
}

fn routes()-> impl Bundle {
	(children![
		pages_routes(),
		docs_routes(),
		actions_routes(),
	],
		// this is placed last to ensure it runs after all handlers
		RouteHandler::layer(|| {
			let mut state = AppState::get();
			state.num_requests += 1;
			AppState::set(state);
		}),

)
}