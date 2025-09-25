use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		<ImageGenerator client:load/>
	}
}
