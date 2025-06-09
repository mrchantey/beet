// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(more_qualified_paths)]
use beet_template::as_beet::*;
use bevy::prelude::*;

#[test]
fn works() {
	rsx! {<HelloWorld/>}
		.xmap(bundle_to_html)
		.xpect()
		.to_be("<div>hello</div>");
}

#[template]
fn HelloWorld() -> impl Bundle {
	rsx! {<div>hello</div>}
}
