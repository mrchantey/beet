#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(not(target_arch = "wasm32"))]
use beet_rsx::as_beet::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn children() {
	use std::borrow::Cow;


	let temp_val: Cow<'static, str> = "Hello, World!".into();
	// more than 12 items uses builder instead of related!
	App::new().world_mut().spawn(rsx! {
		<div>
			<br>{temp_val}</br>
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
			<br />
		</div>
	});
}
