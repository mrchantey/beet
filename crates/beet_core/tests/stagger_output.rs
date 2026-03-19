use beet_core::prelude::*;

#[beet_core::test]
async fn test1() { time_ext::sleep_millis(0).await; }
#[beet_core::test]
async fn test2() { time_ext::sleep_millis(500).await; }
#[beet_core::test]
async fn test3() { time_ext::sleep_millis(1000).await; }
#[beet_core::test]
async fn test4() { time_ext::sleep_millis(1500).await; }
