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

	app.add_plugins((
		BuildPlugin {
			skip_load_workspace: true,
			..default()
		},
		// TemplatePlugin::default(),
	))
	.insert_resource(BuildFlags::only(BuildFlag::FileSnippets));

	let entity = app
		.world_mut()
		.spawn((HtmlDocument, rsx! {
			<Roundtrip/>
		}))
		.id();
	app.update();
	app.world_mut()
		.run_system_once_with(render_fragment, entity)
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
