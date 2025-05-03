//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]
#[cfg(target_arch = "wasm32")]
use sweet_test::as_sweet::*;

#[sweet_test::test]
#[ignore = "it returns error"]
async fn returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn dummy2() {
	sweet_utils::sleep_secs(1).await;
	panic!("waddup")
}
#[sweet_test::test]
// #[should_panic]
async fn dummy3() { sweet_utils::sleep_secs(1).await; }
#[sweet_test::test]
// #[should_panic]
async fn dummy4() { sweet_utils::sleep_secs(1).await; }
#[sweet_test::test]
#[should_panic]
async fn dummy5() {
	sweet_utils::sleep_secs(1).await;
	panic!("whaya");
}
