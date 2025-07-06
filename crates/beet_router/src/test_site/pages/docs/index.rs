use crate::as_beet::*;
use crate::test_site::*;

pub fn get() -> impl Bundle {
	// rsx! { <div>{"party time!"}</div> }
	rsx! { <PageLayout style:cascade title="foobar">party time!</PageLayout> }
}
