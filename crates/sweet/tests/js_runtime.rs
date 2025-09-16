#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_utils::prelude::*;
use sweet::prelude::*;

#[test]
fn cwd() {
	let dir = js_runtime::cwd();
	assert!(dir.contains("sweet"));
}

#[test]
#[ignore = "take hook shenanigans"]
// #[should_panic]
fn panic_to_error() {
	let mut f = || -> Result<(), String> { panic!("it panicked") };
	let result = js_runtime::panic_to_error(&mut f);
	assert!(
		format!("{:?}", result)
			.starts_with("Err(JsValue(RuntimeError: unreachable")
	);
}

#[test]
fn read_file() {
	assert!(js_runtime::read_file("foobar").is_none());
	assert!(js_runtime::read_file("Cargo.toml").is_some());
}

#[test]
fn sweet_root() {
	let root = js_runtime::sweet_root().unwrap().replace("\\", "/");
	assert!(root.ends_with("beet/"));
}

#[test]
fn env_access() {
	js_runtime::env_all().length().xpect_greater_or_equal_to(1);
	js_runtime::env_var("SWEET_ROOT")
		.unwrap()
		.xpect_ends_with("beet/");
}
