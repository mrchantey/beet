// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx::as_beet::*;
use sweet::prelude::*;
use bevy::prelude::*;

#[test]
fn works() {
	rsx! {<HelloWorld/>}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<div>hello</div>");
}

#[template]
fn HelloWorld(a:u32) -> impl Bundle {
	rsx! {<div>hello</div>}
}
