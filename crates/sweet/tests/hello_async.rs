//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use bevy::tasks::futures_lite::future::yield_now;

#[sweet::test]
#[ignore = "it returns error"]
async fn returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn dummy2() {
	yield_now().await;
	panic!("waddup")
}
#[sweet::test]
// #[should_panic]
async fn dummy3() { yield_now().await; }
#[sweet::test]
// #[should_panic]
async fn dummy4() { yield_now().await; }
#[sweet::test]
#[should_panic]
async fn dummy5() {
	yield_now().await;
	panic!("whaya");
}
