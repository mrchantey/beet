#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// use beet_dom::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;


#[test]
fn css() {
	HtmlDocument::parse_bundle(rsx! {
		<div>
			hello world
		<style>
					div { color: red; }
		</style>
		</div>
	})
	.xpect_snapshot();
}
#[test]
fn fs_src() {
	HtmlDocument::parse_bundle(rsx! { <style src="./test_file.css" /> })
		.xpect_snapshot();
}
