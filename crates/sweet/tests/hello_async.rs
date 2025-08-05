//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(target_arch = "wasm32")]
use sweet::prelude::*;

#[sweet::test]
#[ignore = "it returns error"]
async fn returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn dummy2() {
	beet_utils::prelude::sleep_secs(1).await;
	panic!("waddup")
}
#[sweet::test]
// #[should_panic]
async fn dummy3() { beet_utils::prelude::sleep_secs(1).await; }
#[sweet::test]
// #[should_panic]
async fn dummy4() { beet_utils::prelude::sleep_secs(1).await; }
#[sweet::test]
#[should_panic]
async fn dummy5() {
	beet_utils::prelude::sleep_secs(1).await;
	panic!("whaya");
}
