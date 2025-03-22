use beet::prelude::*;


#[cfg(feature = "setup")]
fn main() {
	FileGroupConfig::new(app_cx!())
		.add_group(TreeFileGroup::new("routes"))
		.export();
}


#[cfg(not(feature = "setup"))]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::prelude::routes::collect())
		// .add_plugin(Router::new)
		.run();
}
