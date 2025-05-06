use crate::as_beet::*;
use crate::test_site::*;


pub fn get() -> WebNode {
	// rsx! { <div>{"party time!"}</div> }
	rsx! { <PageLayout scope:cascade title="foobar">party time!</PageLayout> }
}
