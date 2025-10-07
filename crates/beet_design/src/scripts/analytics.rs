use crate::prelude::*;



/// Basic analytics
#[template]
pub fn Analytics() -> impl Bundle {
	let analytics = beet_net::prelude::ANALYTICS_JS.to_string();
	rsx! {
		<script {InnerText(analytics)}/>
	}
}
