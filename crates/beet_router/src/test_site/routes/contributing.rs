#![allow(unused)]
use crate::test_site::*;
use beet_rsx::as_beet::beet;
use beet_rsx::prelude::*;

pub fn get() -> RsxRoot {
	rsx! { <PageLayout title="Beet".into()>party time dude!</PageLayout> }
}
