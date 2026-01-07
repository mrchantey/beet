// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_rsx::prelude::*;

#[test]
fn hello() {
	#[template]
	fn div() -> impl Bundle { (ElementNode::open(), NodeTag::new("div")) }

	#[template]
	fn hello(name: String, r#type: String) -> impl Bundle {
		bsx! { <div>hello {name}</div> }
	}
	bsx! { <hello name="bill" type="foo" /> }
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<div>hello bill</div>");
}
