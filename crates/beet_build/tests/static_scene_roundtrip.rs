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
use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;

#[test]
fn expressions() {
	let scene = build_scene(quote! {
		<div>
		<button key={placeholder}>Click me</button>
		<span>"The value is "{placeholder}</span>
		</div>
	});
	expect(&scene)
		.to_contain("MacroIdx")
		.to_contain("NodeTag")
		.to_contain("ExprIdx")
		.to_contain("StaticNodeRoot");

	// println!(
	// 	"children: {:#?}",
	// 	build_app.component_names_related::<Children>(_entity)
	// );
	// println!("Exported Scene:\n{}", scene);

	apply_and_render(&scene, rsx! {
		<button key={1}> this will be replaced {2}</button>
	})
	.xpect()
	.to_be_str(
		"<div><button key=\"1\">Click me</button><span>The value is 2</span></div>",
	);
}
#[test]
fn style() {
	let scene = build_scene(quote! {
		<div>
			<h1>Roundtrip Test</h1>
		</div>
		<style>
			h1{font-size: 1px;}
		</style>
	});
	apply_and_render(&scene, rsx! {"placeholder"})
		.xpect()
		.to_be(
			"<div data-beet-style-id-0><h1 data-beet-style-id-0>Roundtrip Test</h1></div><style>h1[data-beet-style-id-0] {\n  font-size: 1px;\n}\n</style>",
		);
}


#[template]
fn MyTemplate(initial: u32) -> impl Bundle {
	rsx! {
		<div>
			<h1>"value: "{initial}</h1>
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

fn build_scene(tokens: TokenStream) -> String {
	let mut app = App::new();
	app.add_plugins((NodeTokensPlugin, StaticScenePlugin));
	let _entity = app
		.world_mut()
		.spawn((StaticNodeRoot, common_idx(), RstmlTokens::new(tokens)))
		.id();
	app.update();

	app.build_scene()
}

fn apply_and_render(scene: &str, bundle: impl Bundle) -> String {
	let mut app = App::new();
	app.add_plugins(TemplatePlugin);
	app.load_scene(scene).unwrap();

	let entity = app.world_mut().spawn(bundle).insert(common_idx()).id();

	// app.world_mut().spawn((
	// 	OnSpawnTemplate::new(|_| {
	// 		panic!("dsds");
	// 	}),
	// 	MacroIdx::new_file_line_col(file!(), line!(), column!()),
	// ));
	app.update();

	app.world_mut()
		.run_system_once_with(render_fragment, entity)
		.unwrap()
}
