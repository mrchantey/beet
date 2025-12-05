// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx::prelude::*;
use beet_core::prelude::*;
use sweet::prelude::*;

#[test]
fn hello() {
	#[template]
	fn hello(name: String, r#type: String) -> impl Bundle {
		rsx! { <div>hello {name}</div> }
	}
	rsx! { <Hello name="bill" type="foo" /> }
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<div>hello bill</div>");
}
#[test]
fn entity_id() {
	#[template]
	fn EntityId(entity: Entity) -> impl Bundle {
		rsx! { <div>hello {entity.to_string()}</div> }
	}
	rsx! { <EntityId /> }
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<div>hello 4v0</div>");
}


#[test]
fn result() {
	#[template]
	fn ReturnsResult() -> Result<impl Bundle> {
		rsx! {
			<div>
				<slot />
			</div>
		}
		.xok()
	}

	rsx! { <ReturnsResult>howdy</ReturnsResult> }
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<div>howdy</div>");
}
