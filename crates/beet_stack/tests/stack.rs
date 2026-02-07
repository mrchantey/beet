#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_core::prelude::*;
use beet_stack::prelude::*;




#[test]
fn foo() {
	let mut world = StackPlugin::world();

	let _root = world.spawn((Card, children![
		render_markdown(),
		counter(),
		calculator()
	]));
	// TODO - serve via cli commands
}


fn counter() -> impl Bundle {
	let value = FieldRef::new("count").init_with(Value::I64(0));

	(Card, PathPartial::new("counter"), children![
		render_markdown(),
		increment(value)
	])
}


fn calculator() -> impl Bundle {
	let rhs = FieldRef::new("rhs").init_with(Value::I64(0));
	// let lhs = FieldRef::new("lhs").init_with(Value::I64(0));

	(Card, PathPartial::new("calculator"), children![
		render_markdown(),
		add(rhs)
	])
}
