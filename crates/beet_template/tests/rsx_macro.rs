#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(not(target_arch = "wasm32"))]
use beet_template::as_beet::*;
#[cfg(not(target_arch = "wasm32"))]
use sweet::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn rsx_macro() {
	let (get, set) = signal(String::new());


	App::new()
		.world_mut()
		.spawn(rsx! {<button onclick=move|ev|set(ev.value())>click me</button>})
		.trigger(OnClick::new(MockEvent::new("foo")));
	get().xpect().to_be("foo");
}
