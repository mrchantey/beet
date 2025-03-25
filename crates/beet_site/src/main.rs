use beet::prelude::*;

#[cfg(feature = "setup")]
fn main() {
	use beet::exports::WorkspacePathBuf;

	FileGroupConfig::new(app_cx!())
		.add_group(TreeFileGroup::new(WorkspacePathBuf::new(
			"crates/beet_site/src/routes",
		)))
		// ensures design mockups are regenerated on reload
		// .add_group(beet::design::prelude::mockups())
		.export();
}


#[cfg(not(feature = "setup"))]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::prelude::routes::collect())
		// .add_plugin(Router::new)
		.run();
}
