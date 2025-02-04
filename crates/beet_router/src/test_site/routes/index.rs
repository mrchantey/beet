#![allow(unused)]
use crate::prelude::*;
use crate::test_site::*;
use beet_rsx as beet;
use beet_rsx::prelude::*;

pub fn get(state: DefaultAppState) -> RsxRoot {
	rsx! {
		<PageLayout title=state.app_name.into()>
				party time!
		</PageLayout>
	}
}
