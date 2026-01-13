#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;

#[test]
fn var() { env_ext::var("WORKSPACE_ROOT").unwrap().xpect_ends_with("beet/"); }

#[test]
fn vars_filtered() {
	// implicitly tests `vars()`
	let filter = GlobFilter::default().with_include("WORKSPACE_ROOT");
	let vars = env_ext::vars_filtered(filter);
	vars.len().xpect_eq(1);
	vars[0].0.xref().xpect_eq("WORKSPACE_ROOT");
	vars[0].1.xref().xpect_ends_with("beet/");
}
