#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_core::testing;

#[beet_core::test]
async fn test1() { time_ext::sleep_millis(0).await; }
#[beet_core::test]
async fn test2() { time_ext::sleep_millis(500).await; }
#[beet_core::test]
async fn test3() { time_ext::sleep_millis(1000).await; }
#[beet_core::test]
async fn test4() { time_ext::sleep_millis(1500).await; }
