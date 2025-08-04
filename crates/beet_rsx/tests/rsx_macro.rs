#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(not(target_arch = "wasm32"))]
use beet_rsx::as_beet::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use sweet::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn rsx_macro() {
	let (get, set) = signal(String::new());

	let mut app = App::new();
	app.add_plugins(ApplySnippetsPlugin);
	let world = app.world_mut();
	let button = world
		.spawn(rsx! {<button onclick=move|ev|set(ev.value())>click me</button>})
		.get::<Children>()
		.unwrap()[0];
	world.run_schedule(ApplySnippets);
	world
		.entity_mut(button)
		.trigger(OnClick::new(MockEvent::new("foo")));
	get().xpect().to_be("foo");
}


#[test]
fn inner_text() {
	let code = "let foo = {bar};";
	rsx! {<code inner:text=code />}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<code>let foo = {bar};</code>");
}
