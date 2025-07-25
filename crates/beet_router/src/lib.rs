//! This crate is for patterns that are used in both server and client
//! like Client Islands and Server Actions.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]

pub mod prelude {}