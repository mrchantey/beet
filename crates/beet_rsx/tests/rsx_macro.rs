#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(not(target_arch = "wasm32"))]
use beet_rsx::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_utils::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use sweet::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn reactivity() {
	let (get, set) = signal(String::new());

	let mut app = App::new();
	app.add_plugins(ApplySnippetsPlugin);
	let world = app.world_mut();
	let button = world
		.spawn(
			rsx! { <button onclick=move |ev| set(ev.value())>click me</button> },
		)
		.get::<Children>()
		.unwrap()[0];
	world.run_schedule(ApplySnippets);
	world
		.entity_mut(button)
		.trigger(OnClick(MockEvent::new("foo")));
	get().xpect_eq("foo");
}


#[test]
fn inner_text() {
	let code = "let foo = {bar};";
	rsx! { <code inner:text=code /> }
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<code>let foo = {bar};</code>");
}



#[cfg(not(target_arch = "wasm32"))]
#[test]
fn r#ref() {
	let (get, set) = signal(Entity::PLACEHOLDER);

	let mut app = App::new();
	app.add_plugins(ApplySnippetsPlugin);
	let world = app.world_mut();
	let div = world
		.spawn(rsx! { <div ref=set /> })
		.get::<Children>()
		.unwrap()[0];
	get().xpect_eq(Entity::PLACEHOLDER);
	world.run_schedule(ApplySnippets);
	get().xpect_eq(div);
}
