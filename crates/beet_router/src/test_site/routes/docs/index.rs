#![allow(unused)]
use crate::prelude::*;
use crate::test_site::*;
use beet_rsx::as_beet::*;
use beet_rsx::prelude::*;

pub fn get(state: DefaultAppState) -> RsxRoot {
	// rsx! { <div>{"party time!"}</div> }
	rsx! { <PageLayout title=state.app_name.into()>party time!</PageLayout> }
}
