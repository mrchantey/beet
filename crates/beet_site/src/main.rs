use beet::prelude::*;
use beet_site::prelude::routes;

fn main() {
	// #[cfg(not(feature = "setup"))]
	AppRouter::new(root_cx!())
		.add_collection(routes::collect_file_routes)
		// .add_plugin(Router::new)
		.run();

	// #[cfg(feature = "setup")]
	// BeetApp::new(root_cx!()).run();
}
