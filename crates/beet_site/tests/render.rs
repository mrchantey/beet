#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
// #[ignore = "changes too often"]
async fn docs() {
	// let text = Router::new_bundle(routes_bundle)
	// 	.oneshot("/docs")
	// 	.await
	// 	.text()
	// 	.await
	// 	.unwrap();
	// println!("text: {text}");
	// empty without snippets.ron?

	Router::new_bundle(routes_bundle)
		.oneshot("/docs")
		.await
		.text()
		.await
		.unwrap()
		.xpect()
		.to_contain("Docs");
}
