#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
async fn works() {
	AppRouter::test()
		.add_plugins(PagesPlugin)
		.render_route(&"/".into())
		.await
		.unwrap()
		.xpect()
		.to_be("");
}
