#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![cfg(target_arch = "wasm32")]
// #[cfg(target_arch = "wasm32")]
// use beet_core::prelude::*;
// #[cfg(target_arch = "wasm32")]
// use sweet::prelude::*;

// #[cfg(target_arch = "wasm32")]
#[test]
fn foo() {}
// fn cwd() {
// 	let dir = js_runtime::cwd();
// 	assert!(dir.contains("sweet"));
// }

// #[cfg(target_arch = "wasm32")]
// #[test]
// #[ignore = "take hook shenanigans"]
// // #[should_panic]
// fn panic_to_error() {
// 	let f = || -> Result<(), String> { panic!("it panicked") };
// 	let result = js_runtime::panic_to_error(f);
// 	assert!(
// 		format!("{:?}", result)
// 			.starts_with("Err(JsValue(RuntimeError: unreachable")
// 	);
// }

// #[cfg(target_arch = "wasm32")]
// #[test]
// fn read_file() {
// 	assert!(js_runtime::read_file("foobar").is_none());
// 	assert!(js_runtime::read_file("Cargo.toml").is_some());
// }

// #[cfg(target_arch = "wasm32")]
// #[test]
// fn sweet_root() {
// 	let root = js_runtime::sweet_root().unwrap().replace("\\", "/");
// 	assert!(root.ends_with("beet/"));
// }

// #[cfg(target_arch = "wasm32")]
// #[test]
// fn env_access() {
// 	js_runtime::env_all().length().xpect_greater_or_equal_to(1);
// 	js_runtime::env_var("SWEET_ROOT")
// 		.unwrap()
// 		.xpect_ends_with("beet/");
// }
