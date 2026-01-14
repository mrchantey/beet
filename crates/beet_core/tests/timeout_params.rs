#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Integration tests for per-test timeout parameters
//!
//! This demonstrates the `timeout_ms` parameter on `#[beet_core::test]` which allows
//! per-test timeout configuration, overriding the suite-level timeout.
use beet_core::testing;



/// Test with custom timeout - async function
/// This test has a 100ms timeout which is plenty for a 10ms sleep
#[beet_core::test(timeout_ms = 100)]
async fn timeout_param_async_passes() {
	beet_core::time_ext::sleep_millis(10).await;
	assert_eq!(2 + 2, 4);
}

/// Test with custom timeout - sync wrapper for async body
/// Demonstrates that timeout_ms works with sync tests that register async bodies
#[beet_core::test(timeout_ms = 500)]
fn timeout_param_sync_with_async_body() {
	beet_core::testing::register_test(Default::default(), async {
		beet_core::time_ext::sleep_millis(10).await;
		assert_eq!(2 + 2, 4);
	});
}

/// Test without custom timeout - uses suite default
/// When no timeout_ms is specified, the suite-level timeout applies (default 5000ms)
#[beet_core::test]
async fn timeout_uses_default_when_not_specified() {
	beet_core::time_ext::sleep_millis(10).await;
	assert_eq!(2 + 2, 4);
}

/// Test with very generous timeout
/// Demonstrates that tests can have longer timeouts than the suite default
#[beet_core::test(timeout_ms = 10000)]
async fn timeout_param_longer_than_default() {
	beet_core::time_ext::sleep_millis(50).await;
	assert_eq!(1 + 1, 2);
}
