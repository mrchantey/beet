use beet::prelude::*;
use beet_site::prelude::routes;

#[rustfmt::skip]
fn main() { 
	BeetApp::new(root_cx!())
		.add_collection(routes::collect_file_routes)
		// .add_plugin(Router::new)
		.run(); 
}
