use crate::prelude::*;





pub fn routes()-> impl Bundle {
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