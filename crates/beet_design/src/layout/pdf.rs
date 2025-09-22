use crate::prelude::*;


#[template]
pub fn PageBreak() -> impl Bundle {
	rsx! {
		<div></div>
		<style>
		@media print {
			/*
			h2 {
				break-before: page;
			}*/

			div {
				break-after: page;
			}
		}
		</style>
	}
}
