#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
async fn docs() {
	(MinimalPlugins, RouterPlugin, server_plugin)
		.into_world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.oneshot("/docs")
		.await·
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains("docs");
}
