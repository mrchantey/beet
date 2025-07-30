//! An integration test for lang snippets roudtrip,
//! and also a demostration of using BuildPlugin and ApplyDirectives together
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_build::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[test]
fn works() {
	let mut app = App::new();

	app.add_plugins((BuildPlugin::default(), ApplyDirectivesPlugin::default()))
		.insert_resource(BuildFlags::only(BuildFlag::ExportSnippets))
		.insert_resource(TemplateFlags::None);

	let entity = app
		.world_mut()
		.spawn((HtmlDocument, rsx! {
			<Roundtrip/>
		}))
		.id();
	app.world_mut().run_schedule(BuildSequence);
	app.world_mut().run_schedule(ApplyDirectives);
	// app.update();
	app.world_mut()
		.run_system_cached_with(render_fragment, entity)
		.unwrap()
		.xpect()
		.to_be_str("<!DOCTYPE html><html><head><style>h1[data-beet-style-id-0] {\n  font-size: 1px;\n}\n</style></head><body><div data-beet-style-id-0><h1 data-beet-style-id-0>Roundtrip Test</h1></div></body></html>");
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
