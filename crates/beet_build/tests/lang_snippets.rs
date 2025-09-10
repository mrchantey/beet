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
		.insert_resource(BuildFlags::only(BuildFlag::ExportSnippets));

	let entity = app
		.world_mut()
		.spawn((HtmlDocument, rsx! {
			<Roundtrip/>
		}))
		.id();
	app.world_mut().run_schedule(ApplySnippets);
	app.world_mut().run_schedule(BuildSequence);
	app.world_mut().run_schedule(ApplyDirectives);
	// app.update();
	app.world_mut()
		.run_system_cached_with(render_fragment, entity)
		.unwrap()
		.xpect_snapshot();
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
