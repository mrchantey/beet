use crate::prelude::*;




/// Basic analytics
#[template]
pub fn Analytics() -> impl Bundle {
	rsx! {
		<script src = "../../../../crates/beet_net/src/object_storage/analytics.js"/>
	}
}
