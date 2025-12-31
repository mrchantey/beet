//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
use bevy::tasks::futures_lite::future::yield_now;

#[test]
#[ignore]
fn its_ignored_sync() { panic!("foo") }
#[test]
#[should_panic = "cos its fun"]
fn it_panics_sync() { panic!("foo") }

#[sweet::test]
async fn it_passes() {}
#[sweet::test]
async fn it_returns_ok() -> Result<(), String> { Ok(()) }
#[test]
#[ignore = "it returns error"]
fn it_returns_err() -> Result<(), String> { Err("foo".to_string()) }

#[sweet::test]
#[ignore = "it returns error"]
async fn it_returns_err_async() -> Result<(), String> { Err("foo".to_string()) }

#[sweet::test]
#[should_panic]
async fn it_panics() { panic!("foo") }

// #[cfg(target_arch = "wasm32")]
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
#[should_panic]
async fn it_tokio_waits_then_panics() {
	yield_now().await;
	panic!("waddup")
}
#[sweet::test]
async fn it_sleeps() { yield_now().await; }

#[sweet::test]
#[should_panic]
async fn it_sleeps_then_panics() {
	yield_now().await;
	panic!("waddup")
}
