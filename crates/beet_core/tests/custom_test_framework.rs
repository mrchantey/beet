//! Verifies that the existing `custom_test_framework` behavior still works
//! when the `custom_test_framework` feature is enabled.
//!
//! This test file is only compiled when the feature is active (nightly).
#![cfg(feature = "custom_test_framework")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner_nightly))]

use beet_core::prelude::*;

#[test]
fn sync_pass() { 2.xpect_eq(2); }

#[test]
fn sync_result_ok() -> Result<(), String> { Ok(()) }

#[test]
#[ignore]
fn ignored_test() {
	panic!("should not run");
}

#[test]
#[should_panic]
fn expected_panic() {
	panic!("this is expected");
}

#[test]
fn async_via_register() {
	register_test(TestCaseParams::new(), async {
		time_ext::sleep_millis(5).await;
		Ok(())
	});
}

#[beet_core::test]
fn beet_sync() { true.xpect_true(); }

#[beet_core::test]
async fn beet_async() { time_ext::sleep_millis(5).await; }

#[beet_core::test(timeout_ms = 5000)]
async fn beet_async_with_timeout() { time_ext::sleep_millis(5).await; }
