use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl Bundle {
	rsx! {
		<BeetSidebarLayout>
			<div>yup its docs</div>
		</BeetSidebarLayout>
	}
}
