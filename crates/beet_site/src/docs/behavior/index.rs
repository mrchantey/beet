use beet::prelude::*;

pub fn get() -> impl IntoHtml {
	rsx! {
		{include_str!("../../../../beet_flow/README.md")}
	}
}
