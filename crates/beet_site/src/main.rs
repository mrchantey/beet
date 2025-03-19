use beet::prelude::*;
use beet_site::prelude::routes;

fn main() {
	#[cfg(feature = "setup")]
	FileGroupConfig::new(root_cx!())
		.add_group(TreeFileGroup::new("routes"))
		.export();

	#[cfg(not(feature = "setup"))]
	AppRouter::new(app_cx!())
		.add_collection(routes::collect_file_routes)
		// .add_plugin(Router::new)
		.run();
}
