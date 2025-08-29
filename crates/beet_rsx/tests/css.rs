#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx::as_beet::*;
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
	.xpect()
	.to_be_snapshot();
}
#[test]
fn fs_src() {
	HtmlDocument::parse_bundle(rsx! { <style src="./test_file.css" /> })
		.xpect()
		.to_be_snapshot();
}
