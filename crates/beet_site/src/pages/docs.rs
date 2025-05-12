use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> WebNode {
	rsx! {
		<BeetSidebarLayout>
			<div>yup its docs</div>
		</BeetSidebarLayout>
	}
}
