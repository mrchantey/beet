//! This test checks the full serde roundtrip of applying a static node
//! to an instance, using two seperate apps.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_build::prelude::*;
use beet_parse::prelude::*;
use beet_template::as_beet::*;
// use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use quote::quote;
use sweet::prelude::*;

#[test]
fn works() {
	let mut build_app = App::new();

	build_app.add_plugins((NodeTokensPlugin, StaticScenePlugin));


	let static_node = build_app
		.world_mut()
		.spawn((
			StaticNodeRoot,
			RstmlTokens::new(quote! {
				<div>
				<button key={value}>Click me</button>
				<span>The value is {value}</span>
				</div>
			}),
		))
		.id();
	build_app.update();
	println!("children: {:#?}", build_app.component_names_related::<Children>(static_node));

	let scene = build_app.build_scene();
	expect(&scene).to_contain("MacroIdx");

	println!("Exported Scene:\n{}", scene);
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
