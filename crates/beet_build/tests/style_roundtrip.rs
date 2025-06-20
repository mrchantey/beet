#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_build::prelude::*;
use beet_template::as_beet::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use sweet::prelude::*;

#[test]
fn works() {
	let mut app = App::new();

	app.add_plugins((BuildTemplatesPlugin, TemplatePlugin));


	let entity = app
		.world_mut()
		.spawn(HtmlDocument::wrap_bundle(rsx! {
			<Roundtrip/>
		}))
		.id();
	app.update();
	app
		.world_mut()
		.run_system_once_with(render_fragment, entity)
		.unwrap().xpect().to_be(
			"<!DOCTYPE html><html><head><style>h1[data-beet-style-id-0] {\n  font-size: 1px;\n}\n</style></head><body><div data-beet-style-id-0><h1 data-beet-style-id-0>Roundtrip Test</h1></div></body></html>",
		);
}




#[template]
fn Roundtrip() -> impl Bundle {
	rsx! {
		<div>
			<h1>Roundtrip Test</h1>
		</div>
		<style>
			h1{font-size: 1px;}
		</style>
	}
}
