#![allow(unused)]
use crate::as_beet::*;
use crate::test_site::*;

pub fn get() -> WebNode {
	// rsx! { <div>party time dude!</div> }
	rsx! { <PageLayout title="Test Site">party time dude!</PageLayout> }
}
