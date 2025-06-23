#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
async fn works() {
	let sidebar_tree =
		route_path_tree().xpipe(RoutePathTreeToSidebarTree::default());
	sidebar_tree[0].display_name.xref().xpect().to_be("Docs");
	sidebar_tree[0].children[0].display_name.xref().xpect().to_be("Testing");

	// println!("{:#?}", sidebar_tree);
}
