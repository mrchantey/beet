use crate::prelude::*;




/// Basic analytics
#[template]
pub fn Analytics() -> impl Bundle {
	rsx! {
		<script {InnerText(ANALYTICS_JS.into())}/>
	}
}
