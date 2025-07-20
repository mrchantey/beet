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
	let button = app
		.world_mut()
		.spawn(rsx! {<button onclick=move|ev|set(ev.value())>click me</button>})
		.get::<Children>()
		.unwrap()[0];
	app.world_mut()
		.run_system_cached(apply_static_rsx)
		.unwrap()
		.unwrap();
	app.world_mut()
		.entity_mut(button)
		.trigger(OnClick::new(MockEvent::new("foo")));
	get().xpect().to_be("foo");
}
