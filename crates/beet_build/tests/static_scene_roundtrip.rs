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
	roundtrip_to_html(
		rsx! {
			<button key={1}> this will be replaced {2}</button>
		},
		quote! {
			<div>
			<button key={placeholder}>Click me</button>
			<span>"The value is "{placeholder}</span>
			</div>
		},
	)
	.xpect()
	.to_be_str(
		"<div><button key=\"1\">Click me</button><span>The value is 2</span></div>",
	);
}
#[test]
fn style() {
	roundtrip_to_html(
		rsx! {"placeholder"},
		quote! {
			<div>
				<h1>Roundtrip Test</h1>
			</div>
			<style>
				h1{font-size: 1px;}
			</style>
		},
	)
	.xpect()
	.to_be_str(
		"<div><button key=\"1\">Click me</button><span>The value is 2</span></div>",
	);
}


fn roundtrip_to_html(bundle: impl Bundle, tokens: TokenStream) -> String {
	let scene = build_scene(tokens);
	//
	// simulated serde boundary
	//
	apply_scene(bundle, &scene)
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
	// println!("Exported Scene:\n{}", scene);

	scene
}

fn apply_scene(bundle: impl Bundle, scene: &str) -> String {
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
