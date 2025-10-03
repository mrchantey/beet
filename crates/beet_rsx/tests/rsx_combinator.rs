#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_rsx::prelude::*;
use sweet::prelude::*;

#[test]
fn rsx_combinator() {
	let (get, set) = signal(String::new());

	let mut world = World::new();
	let button = world
		.spawn(rsx_combinator! {"<button onclick={move|ev|set(ev.value())}>click me</button>"})
		.get::<Children>()
		.unwrap()[0];
	world
		.entity_mut(button)
		.trigger_target(OnClick(MockEvent::new("foo")));
	get().xpect_eq("foo");
}
