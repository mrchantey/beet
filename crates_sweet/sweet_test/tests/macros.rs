//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]
use anyhow::Result;
#[cfg(target_arch = "wasm32")]
use sweet_test::as_sweet::*;

#[test]
#[ignore]
fn its_ignored_sync() { panic!("foo") }
#[test]
#[should_panic = "cos its fun"]
fn it_panics_sync() { panic!("foo") }

#[sweet_test::test]
async fn it_passes() {}
#[sweet_test::test]
async fn it_returns_ok() -> Result<(), String> { Ok(()) }
#[test]
#[ignore = "it returns error"]
fn it_returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[sweet_test::test]
#[ignore = "it returns error"]
async fn it_returns_err_async() -> Result<(), String> { Err("foo".to_string()) }

#[sweet_test::test]
#[should_panic]
async fn it_panics() { panic!("foo") }

// #[cfg(target_arch = "wasm32")]
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn it_tokio_waits_then_panics() {
	sweet_utils::sleep_secs(1).await;
	panic!("waddup")
}
#[sweet_test::test]
// #[should_panic]
async fn it_sleeps() { sweet_utils::sleep_secs(1).await; }

#[sweet_test::test]
#[should_panic]
async fn it_sleeps_then_panics() {
	sweet_utils::sleep_secs(1).await;
	panic!("waddup")
}
