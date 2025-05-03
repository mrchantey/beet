//! example configuration for a test, just two lines and you're good to go
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]
use sweet_test::prelude::*;

#[test]
fn it_succeeds() { assert!(true) }
#[test]
fn it_succeeds2() { assert!(true) }
#[test]
#[should_panic]
fn it_succeeds3() { expect(true).to_be_false() }


// #[test]
// #[should_panic]
// fn it_panics() { panic!("foo") }


#[test]
// #[ignore]
#[should_panic]
// fn it_panics2() {}
fn it_panics2() { panic!("foo") }
