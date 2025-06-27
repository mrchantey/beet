//! This test checks the full serde roundtrip of applying a static node
//! to an instance, using two seperate apps.
//!
//! This could also be considered the integration test for
//! [`apply_static_nodes`]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_build::prelude::*;
use beet_parse::prelude::*;
use beet_template::as_beet::*;
// use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use quote::quote;
use sweet::prelude::*;


#[test]
fn works() {
	let scene = build_scene();
	//
	// simulated serde boundary
	//
	apply_scene(&scene);
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

// create a common idx for matching in apply_static_nodes
fn common_idx() -> MacroIdx {
	MacroIdx::new_file_line_col(file!(), line!(), column!())
}

fn build_scene() -> String {
	let mut app = App::new();
	app.add_plugins((NodeTokensPlugin, StaticScenePlugin));
	let _entity = app
		.world_mut()
		.spawn((
			StaticNodeRoot,
			common_idx(),
			RstmlTokens::new(quote! {
				<div>
				<button key={value}>Click me</button>
				<span>The value is {value}</span>
				</div>
			}),
		))
		.id();
	app.update();

	let scene = app.build_scene();
	expect(&scene)
		.to_contain("MacroIdx")
		.to_contain("NodeTag")
		.to_contain("ExprIdx")
		.to_contain("StaticNodeRoot");

	// println!(
	// 	"children: {:#?}",
	// 	build_app.component_names_related::<Children>(_entity)
	// );
	println!("Exported Scene:\n{}", scene);

	scene
}

fn apply_scene(scene: &str) {
	let mut app = App::new();
	app.add_plugins(TemplatePlugin);
	app.load_scene(scene).unwrap();

	let value = 42;

	let entity = app
		.world_mut()
		.spawn(rsx! {
			<button key={value}> value is {value}</button>
		})
		.insert(common_idx())
		.id();

	app.update();

	let html = app
		.world_mut()
		.run_system_once_with(render_fragment, entity)
		.unwrap();

	println!("Rendered HTML:\n{}", html);
}
