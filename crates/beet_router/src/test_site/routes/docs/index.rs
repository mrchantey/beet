#![allow(unused)]
use crate::prelude::*;
use crate::test_site::*;
use beet_rsx::as_beet::*;
use beet_rsx::prelude::*;

pub fn get() -> RsxRoot {
	// rsx! { <div>{"party time!"}</div> }
	rsx! { <PageLayout title="foobar">party time!</PageLayout> }
}
