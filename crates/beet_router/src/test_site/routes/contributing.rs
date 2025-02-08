#![allow(unused)]
use crate::test_site::*;
use beet_rsx::as_beet::*;
use beet_rsx::prelude::*;

pub fn get() -> RsxRoot {
	// rsx! { <div>party time dude!</div> }
	rsx! { <PageLayout title="Beet".into()>party time dude!</PageLayout> }
}
