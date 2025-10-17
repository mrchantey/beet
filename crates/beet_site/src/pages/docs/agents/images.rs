use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl IntoHtml {
	rsx! {
		<ImageGenerator client:load/>
	}
}
