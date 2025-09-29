#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_rsx::prelude::*;
use sweet::prelude::*;

#[test]
fn rsx_combinator() {
	let (get, set) = signal(String::new());

	let mut app = App::new();
	app.add_plugins(ApplySnippetsPlugin);
	let world = app.world_mut();
	let button = world
		.spawn(rsx_combinator! {"<button onclick={move|ev|set(ev.value())}>click me</button>"})
		.get::<Children>()
		.unwrap()[0];
	world.run_schedule(ApplySnippets);
	world
		.entity_mut(button)
		.auto_trigger(OnClick(MockEvent::new("foo")));
	get().xpect_eq("foo");
}
