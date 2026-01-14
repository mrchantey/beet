#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#[cfg(not(target_arch = "wasm32"))]
use beet_core::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_dom::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_rsx::prelude::*;
#[cfg(not(target_arch = "wasm32"))]


fn is_equal(a: impl Bundle, b: impl Bundle) {
	let mut world = World::new();
	let a = world.spawn(a).insert(SnippetRoot::default()).id();
	let b = world.spawn(b).insert(SnippetRoot::default()).id();
	let a = world
		.component_names_related::<Children>(a)
		.iter_to_string_indented();
	let b = world
		.component_names_related::<Children>(b)
		.iter_to_string_indented();
	a.xpect_str(b);
}


#[test]
fn works() {
	is_equal(
		rsx! {<div>hello</div>},
		rsx_combinator! {"<div>hello</div>"},
	);
	is_equal(
		rsx! {<div>{"hello"}</div>},
		rsx_combinator! {r#"<div>{"hello"}</div>"#},
	);
}
