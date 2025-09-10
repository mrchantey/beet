//! This test checks the full serde roundtrip of applying a static node
//! to an instance, using two seperate apps.
//!
//! This could also be considered the integration test for
//! [`ApplySnippets`]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_build::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use beet_rsx::prelude::*;
use beet_utils::prelude::*;
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
	scene
		.xref()
		.xpect_contains("SnippetRoot")
		.xpect_contains("NodeTag")
		.xpect_contains("ExprIdx")
		.xpect_contains("StaticRoot");

	// println!(
	// 	"children: {:#?}",
	// 	build_app.component_names_related::<Children>(_entity)
	// );
	// println!("Exported Scene:\n{}", scene);

	apply_and_render(&scene, rsx! {
		<button key={1}> this will be replaced {2}</button>
	})
	.xpect_contains(
		"<div><button key=\"1\">Click me</button><span>The value is 2</span></div>",
	);
}
#[test]
#[ignore = "TODO ensure ids are applied to the elements in build_scene"]
fn style() {
	let scene = build_scene(quote! {
		<div>
			<h1>Roundtrip Test</h1>
		</div>
		<style>
			h1{font-size: 1px;}
		</style>
	});
	apply_and_render(&scene, rsx! {"placeholder"}).xpect_str("<!DOCTYPE html><html><head><style>h1[data-beet-style-id-0] {\n  font-size: 1px;\n}\n</style></head><body><div data-beet-style-id-0><h1 data-beet-style-id-0>Roundtrip Test</h1></div></body></html>");
}
#[test]
fn simple_template() {
	#[template]
	fn MyTemplate(initial: u32) -> impl Bundle {
		rsx! {
			<span>"value: "{initial}</span>
		}
	}

	let scene = build_scene(quote! {
	<div>
		<h1>Roundtrip Test</h1>
		<SomeCapitalizedTagToIndicateATemplate/>
	</div>
	});
	apply_and_render(&scene, rsx! {
		<div>
			<MyTemplate initial={1} />
		</div>
	})
	.xpect_contains("<div><h1>Roundtrip Test</h1><span>value: 1</span></div>");
}


#[test]
fn nested_template() {
	let mut app = App::new();
	app.add_plugins(BuildPlugin::default())
		.insert_resource(BuildFlags::None);

	// create root static node
	app.world_mut().spawn((
		StaticRoot,
		common_idx(),
		RstmlTokens::new(quote! {
			<html>
			<SomeCapitalizedTagToIndicateATemplate/>
			</html>
		}),
	));
	// create nested static node
	app.world_mut().spawn((
		StaticRoot,
		common_idx_nested(),
		RstmlTokens::new(quote! {
			<after>"value: "{}</after>
		}),
	));
	app.update();

	let scene = app.build_scene();
	// println!("Exported Scene:\n{}", scene);

	#[template]
	fn NestedTemplate(initial: u32) -> impl Bundle {
		(
			rsx! {
				<span>"value: "{initial}</span>
			},
			OnSpawn::new(|entity| {
				entity.insert(common_idx_nested());
			}),
		)
	}

	apply_and_render(&scene, rsx! {
		<div>
			<NestedTemplate initial={1} />
		</div>
	})
	.xpect_contains("<html><after>value: 1</after></html>");
}




// create a common idx for matching in [`ApplySnippets`]
fn common_idx() -> SnippetRoot {
	SnippetRoot::new_file_line_col(file!(), line!(), column!())
}
// create a common idx for matching in [`ApplySnippets`]
fn common_idx_nested() -> SnippetRoot {
	SnippetRoot::new_file_line_col(file!(), line!(), column!())
}

fn build_scene(tokens: TokenStream) -> String {
	let mut app = App::new();
	app.add_plugins(BuildPlugin::default())
		.insert_resource(BuildFlags::None);
	let _entity = app
		.world_mut()
		.spawn((StaticRoot, common_idx(), RstmlTokens::new(tokens)))
		.id();
	app.update();

	app.build_scene()
}

fn apply_and_render(scene: &str, bundle: impl Bundle) -> String {
	let mut app = App::new();
	app.add_plugins(ApplyDirectivesPlugin);
	app.load_scene(scene).unwrap();

	let root = app
		.world_mut()
		.spawn((HtmlDocument, bundle))
		.insert(common_idx())
		// .spawn(HtmlDocument)
		// .with_children(|parent| {
		// 	parent.spawn(bundle).insert(common_idx());
		// })
		.id();
	app.update();

	app.world_mut()
		.run_system_cached_with(render_fragment, root)
		.unwrap()
}
