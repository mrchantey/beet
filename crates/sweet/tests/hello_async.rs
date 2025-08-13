//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_utils::time_ext::sleep_secs;
#[cfg(target_arch = "wasm32")]
use sweet::prelude::*;

#[sweet::test]
#[ignore = "it returns error"]
async fn returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn dummy2() {
	sleep_secs(1).await;
	panic!("waddup")
}
#[sweet::test]
// #[should_panic]
async fn dummy3() { sleep_secs(1).await; }
#[sweet::test]
// #[should_panic]
async fn dummy4() { sleep_secs(1).await; }
#[sweet::test]
#[should_panic]
async fn dummy5() {
	sleep_secs(1).await;
	panic!("whaya");
}
