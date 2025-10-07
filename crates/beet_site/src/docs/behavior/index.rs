use beet::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		{include_str!("../../../../beet_flow/README.md")}
	}
}
