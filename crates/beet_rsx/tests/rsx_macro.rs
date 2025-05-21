#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

use beet_rsx::as_beet::*;
use bevy::prelude::*;

#[test]
fn rsx_macro() {
	let val = mock_bucket();
	let val2 = val.clone();

	App::new()
		.world_mut()
		.spawn(rsx! {<button onclick={move||val2.call(2)}/>})
		.trigger(OnClick);
	expect(&val).to_have_returned_with(2);
}
