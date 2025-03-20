use beet::prelude::*;

fn main() {
	#[cfg(feature = "setup")]
	FileGroupConfig::new(app_cx!())
		.add_group(TreeFileGroup::new("routes"))
		.export();

	#[cfg(not(feature = "setup"))]
	AppRouter::new(app_cx!())
		.add_collection(beet_site::prelude::routes::collect())
		// .add_plugin(Router::new)
		.run();
}
